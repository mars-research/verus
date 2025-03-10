use crate::ast::{
    CallTarget, CallTargetKind, Expr, ExprX, Fun, Function, FunctionKind, Ident, Krate, Mode, Path,
    Typ, VirErr, WellKnownItem,
};
use crate::ast_util::error;
use crate::def::Spanned;
use air::ast::Span;
use air::scope_map::ScopeMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// We currently do not support trait bounds for traits from other crates
// and we consider methods for traits from other crates to be static.
pub fn demote_foreign_traits(
    path_to_well_known_item: &std::collections::HashMap<Path, WellKnownItem>,
    krate: &Krate,
) -> Result<Krate, VirErr> {
    let traits: HashSet<Path> = krate.traits.iter().map(|t| t.x.name.clone()).collect();
    let func_map: HashMap<Fun, Function> =
        krate.functions.iter().map(|f| (f.x.name.clone(), f.clone())).collect();

    let mut kratex = (**krate).clone();
    for function in &mut kratex.functions {
        /* TODO: this check was broken in earlier versions of this code, and fixing would break
         * some std_specs declarations (for bounds X: Allocator and X: Debug).
         * In the long run, we should probably reenable this check
         * and allow users to declare external traits in order to satisfy this check.
         * In the meantime, omitting this check doesn't cause any soundness issues.
        for bounds in function.x.typ_bounds.iter() {
            let GenericBoundX::Trait(trait_path, _) = &**bounds;
            let our_trait = traits.contains(trait_path);
           if !our_trait {
                return error(
                    &function.span,
                    format!(
                        "cannot use trait {} from another crate as a bound",
                        crate::ast_util::path_as_friendly_rust_name(trait_path)
                    ),
                );
            }
        }
        */

        if let FunctionKind::TraitMethodImpl { method, trait_path, .. } = &function.x.kind {
            let our_trait = traits.contains(trait_path);
            let mut functionx = function.x.clone();
            if our_trait {
                let decl = &func_map[method];
                let mut retx = functionx.ret.x.clone();
                retx.name = decl.x.ret.x.name.clone();
                functionx.ret = Spanned::new(functionx.ret.span.clone(), retx);
            } else {
                if path_to_well_known_item.get(trait_path) == Some(&WellKnownItem::DropTrait) {
                    if !function.x.require.is_empty() {
                        return error(
                            &function.span,
                            "requires are not allowed on the implementation for Drop",
                        );
                    }
                    if !matches!(&function.x.mask_spec, crate::ast::MaskSpec::InvariantOpens(es) if es.len() == 0)
                    {
                        return error(
                            &function.span,
                            "the implementation for Drop must be marked opens_invariants none",
                        );
                    }
                }
                check_modes(function, &function.span)?;
                functionx.kind = FunctionKind::Static;
            }
            *function = Spanned::new(function.span.clone(), functionx);
        }

        if let FunctionKind::ForeignTraitMethodImpl(trait_path) = &function.x.kind {
            let our_trait = traits.contains(trait_path);
            let mut functionx = function.x.clone();
            if our_trait {
                return error(
                    &function.x.proxy.as_ref().unwrap().span,
                    "external_fn_specification can only be used on trait functions when the trait itself is external",
                );
            } else {
                check_modes(function, &function.x.proxy.as_ref().unwrap().span)?;
                functionx.kind = FunctionKind::Static;
            }
            *function = Spanned::new(function.span.clone(), functionx);
        }

        let mut map: ScopeMap<Ident, Typ> = ScopeMap::new();
        *function = crate::ast_visitor::map_function_visitor_env(
            &function,
            &mut map,
            &mut (),
            &|_state, _, expr| demote_one_expr(&traits, expr),
            &|_state, _, stmt| Ok(vec![stmt.clone()]),
            &|_state, typ| Ok(typ.clone()),
        )?;
    }

    Ok(Arc::new(kratex))
}

fn check_modes(function: &Function, span: &Span) -> Result<(), VirErr> {
    if function.x.mode != Mode::Exec {
        return error(span, "function for external trait must have mode 'exec'");
    }
    for param in function.x.params.iter() {
        if param.x.mode != Mode::Exec {
            return error(
                span,
                "function for external trait must have all parameters have mode 'exec'",
            );
        }
    }
    if function.x.ret.x.mode != Mode::Exec {
        return error(
            span,
            "function for external trait must have all parameters have mode 'exec'",
        );
    }
    Ok(())
}

fn get_trait(fun: &Fun) -> Path {
    fun.path.pop_segment()
}

fn demote_one_expr(traits: &HashSet<Path>, expr: &Expr) -> Result<Expr, VirErr> {
    match &expr.x {
        ExprX::Call(
            CallTarget::Fun(
                CallTargetKind::Method(Some((resolved_fun, resolved_typs, impl_paths))),
                fun,
                _typs,
                _impl_paths,
                autospec_usage,
            ),
            args,
        ) if !traits.contains(&get_trait(fun)) => {
            let kind = CallTargetKind::Static;
            let ct = CallTarget::Fun(
                kind,
                resolved_fun.clone(),
                resolved_typs.clone(),
                impl_paths.clone(),
                *autospec_usage,
            );
            Ok(expr.new_x(ExprX::Call(ct, args.clone())))
        }
        _ => Ok(expr.clone()),
    }
}
