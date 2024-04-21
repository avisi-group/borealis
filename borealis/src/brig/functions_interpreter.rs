//! Rust code generation
//!
//! Watch out for `return quote!(...)` in these functions when they build up
//! quotes

use {
    crate::{
        brig::{codegen_ident, codegen_member, codegen_type},
        rudder::{
            BinaryOperationKind, Block, CastOperationKind, ConstantValue, Context, Function,
            PrimitiveType, PrimitiveTypeClass, ShiftOperationKind, Statement, StatementKind,
            Symbol, Type, UnaryOperationKind,
        },
    },
    common::{intern::InternedString, HashSet},
    proc_macro2::{Literal, TokenStream},
    quote::{format_ident, quote, ToTokens},
    std::sync::Arc,
    syn::Ident,
};

pub fn codegen_parameters(parameters: &[Symbol]) -> TokenStream {
    let parameters = [quote!(state: &mut State), quote!(tracer: &T)]
        .into_iter()
        .chain(parameters.iter().map(|sym| {
            let name = codegen_ident(sym.name());
            let typ = codegen_type(sym.typ());
            quote!(#name: #typ)
        }));

    quote! {
        #(#parameters),*
    }
}

pub fn codegen_block(block: Block) -> TokenStream {
    block
        .statements()
        .iter()
        .cloned()
        .map(codegen_stmt)
        .collect()
}

fn get_ident(stmt: &Statement) -> TokenStream {
    format_ident!("{}", stmt.name().to_string()).to_token_stream()
}

pub fn get_block_fn_ident(b: &Block) -> Ident {
    format_ident!("block_{}", b.name().as_ref())
}

//
pub fn codegen_stmt(stmt: Statement) -> TokenStream {
    let stmt_name = format_ident!("{}", stmt.name().to_string());

    let value = match stmt.kind() {
        StatementKind::Constant { value, typ } => {
            let v = match value {
                ConstantValue::UnsignedInteger(v) => {
                    if *typ == Type::u1() {
                        let b = v != 0;
                        quote!(#b)
                    } else {
                        let v = Literal::usize_unsuffixed(v);
                        quote!(#v)
                    }
                }
                ConstantValue::SignedInteger(v) => {
                    let v = Literal::isize_unsuffixed(v);
                    quote!(#v)
                }
                ConstantValue::FloatingPoint(v) => {
                    let v = Literal::f64_unsuffixed(v);
                    quote!(#v)
                }

                ConstantValue::Unit => quote!(()),
            };

            if let Type::Bundled { .. } = &*stmt.typ() {
                let length = Literal::usize_unsuffixed(typ.width_bits());
                quote!(Bundle { value: #v, length: #length })
            } else {
                v
            }
        }
        StatementKind::ReadVariable { symbol, indices } => {
            let var = codegen_ident(symbol.name());

            let indices = indices
                .iter()
                .copied()
                .map(codegen_member)
                .map(|field| quote!(.#field))
                .collect::<TokenStream>();

            quote! {fn_state.#var #indices}
        }
        StatementKind::WriteVariable {
            symbol,
            indices,
            value,
        } => {
            let var = codegen_ident(symbol.name());

            let indices = indices
                .iter()
                .copied()
                .map(codegen_member)
                .map(|field| quote!(.#field))
                .collect::<TokenStream>();

            let value = get_ident(&value);
            quote! {fn_state.#var #indices = #value}
        }
        StatementKind::ReadRegister { typ, offset } => {
            let offset = get_ident(&offset);
            let typ = codegen_type(typ);
            quote! {
                {
                    let value = state.read_register::<#typ>(#offset as isize);
                    tracer.read_register(#offset as usize, value);
                    value
                }
            }
        }
        StatementKind::WriteRegister { offset, value } => {
            let offset = get_ident(&offset);
            let typ = codegen_type(value.typ());
            let value = get_ident(&value);
            quote! {
                {
                    state.write_register::<#typ>(#offset as isize, #value);
                    tracer.write_register(#offset as usize, #value);
                }
            }
        }
        StatementKind::ReadMemory { .. } => quote!(todo!("read-mem")),
        StatementKind::WriteMemory { offset, value } => {
            let offset = get_ident(&offset);

            // find size of value, either bundle.length or in type

            // emit match on this length to create mut pointer

            let (length, value) = match &*value.typ() {
                Type::Primitive(PrimitiveType {
                    element_width_in_bits,
                    ..
                }) => {
                    let value = get_ident(&value);
                    (quote!(#element_width_in_bits), quote!(#value))
                }
                Type::Bundled { .. } => {
                    let value = get_ident(&value);
                    (quote!(#value.length), quote!(#value.value))
                }
                _ => todo!(),
            };

            quote! {
                match #length {
                    8 => unsafe { *(#offset as *mut u8) = #value as u8; },
                    16 => unsafe { *(#offset as *mut u16) = #value as u16; },
                    32 => unsafe { *(#offset as *mut u32) = #value as u32; },
                    64 => unsafe { *(#offset as *mut u64) = #value as u64; },
                    _ => panic!("unsupported length")
                }
            }
        }
        StatementKind::ReadPc => quote!(todo!("read-pc")),
        StatementKind::WritePc { .. } => quote!(todo!("write-pc")),
        StatementKind::BinaryOperation { kind, lhs, rhs } => {
            let mut left = get_ident(&lhs);
            let mut right = get_ident(&rhs);

            // hard to decide whether this belongs, but since it's a Rust issue that u1 is
            // not like other types, casting is a codegen thing
            match (lhs.typ().width_bits(), rhs.typ().width_bits()) {
                // both bools, do nothing
                (1, 1) => (),
                (1, _) => {
                    let typ = codegen_type(rhs.typ());
                    left = quote!(((#left) as #typ));
                }
                (_, 1) => {
                    let typ = codegen_type(lhs.typ());
                    right = quote!(((#right) as #typ));
                }
                // both not bools, do nothing
                (_, _) => (),
            }

            let op = match kind {
                BinaryOperationKind::CompareEqual => quote! { (#left) == (#right) },
                BinaryOperationKind::Add => {
                    quote! { #left.wrapping_add(#right) }
                }
                BinaryOperationKind::Sub => quote! { (#left) - (#right) },
                BinaryOperationKind::Multiply => quote! { (#left) * (#right) },
                BinaryOperationKind::Divide => quote! { (#left) / (#right) },
                BinaryOperationKind::Modulo => quote! { (#left) % (#right) },
                BinaryOperationKind::And => quote! { (#left) & (#right) },
                BinaryOperationKind::Or => quote! { (#left) | (#right) },
                BinaryOperationKind::Xor => quote! { (#left) ^ (#right) },
                BinaryOperationKind::CompareNotEqual => quote! { (#left) != (#right) },
                BinaryOperationKind::CompareLessThan => quote! { (#left) < (#right) },
                BinaryOperationKind::CompareLessThanOrEqual => quote! { (#left) <= (#right) },
                BinaryOperationKind::CompareGreaterThan => quote! { (#left) > (#right) },
                BinaryOperationKind::CompareGreaterThanOrEqual => quote! { (#left) >= (#right) },
            };

            quote! { (#op) }
        }
        StatementKind::UnaryOperation { kind, value } => {
            let value = get_ident(&value);

            match kind {
                UnaryOperationKind::Not => quote! {!#value},
                UnaryOperationKind::Negate => quote! {-#value},
                UnaryOperationKind::Complement => quote! {!#value},
                UnaryOperationKind::Power2 => quote! { (#value).pow2() },
                UnaryOperationKind::Absolute => quote! { (#value).abs() },
            }
        }
        StatementKind::ShiftOperation {
            kind,
            value,
            amount,
        } => {
            let value = get_ident(&value);
            let amount = get_ident(&amount);

            match kind {
                ShiftOperationKind::LogicalShiftLeft => quote! {#value << #amount},
                ShiftOperationKind::LogicalShiftRight => quote! {#value >> #amount},
                ShiftOperationKind::ArithmeticShiftRight => {
                    quote! {#value >> #amount}
                }
                ShiftOperationKind::RotateRight => todo!(),
                ShiftOperationKind::RotateLeft => todo!(),
            }
        }
        StatementKind::Call { target, args, tail } => {
            let ident = codegen_ident(target.name());
            let args = args.iter().map(get_ident);

            if tail {
                quote! {
                    return #ident(state, tracer, #(#args),*)
                }
            } else {
                quote! {
                    #ident(state, tracer, #(#args),*)
                }
            }
        }
        StatementKind::Cast { typ, value, kind } => {
            let source = value.typ();
            let target = typ;
            let ident = get_ident(&value);

            if source == target {
                // todo: check this never happens
                quote! {
                    ((#ident))
                }
            } else if target == Arc::new(Type::u1()) {
                // todo: is special casing booleans like this necessary?
                // todo: this is duplicated in decode
                quote! {
                    ((#ident) != 0)
                }
            } else if target.is_unknown_length_vector() {
                quote!(alloc::vec::Vec::from(#ident))
            } else {
                match kind {
                    CastOperationKind::ZeroExtend => {
                        // safe even if underlying rudder types are smaller than codegen'd rust
                        // types (u7 -> u13 == u8 -> u16)
                        let target = codegen_type(target);
                        quote! {
                            (#ident as #target)
                        }
                    }

                    CastOperationKind::Truncate => {
                        match (&*source, &*target) {
                            (Type::Primitive(_), Type::Primitive(_)) => {
                                assert!(target.width_bits() < source.width_bits());

                                // create mask of length target

                                let mask = Literal::u64_unsuffixed(
                                    1u64.checked_shl(u32::try_from(target.width_bits()).unwrap())
                                        .map(|x| x - 1)
                                        .unwrap_or(!0),
                                );

                                // cast to target type and apply mask to source
                                let target = codegen_type(target);

                                quote!(((#ident as #target) & #mask))
                            }
                            (Type::Bundled { .. }, Type::Bundled { .. }) => {
                                panic!("cannot truncate bundles, target length not known by type system")
                            }
                            _ => todo!(),
                        }
                    }

                    CastOperationKind::SignExtend => todo!(),
                    CastOperationKind::Reinterpret => {
                        let target = codegen_type(target);
                        quote! {
                            (#ident as #target)
                        }
                    }
                    CastOperationKind::Convert => todo!(),
                    CastOperationKind::Broadcast => todo!(),
                }
            }
        }
        StatementKind::Jump { target } => {
            let target = get_block_fn_ident(&target);
            quote! {
               return #target(state, tracer, fn_state);
            }
        }
        StatementKind::Branch {
            condition,
            true_target,
            false_target,
        } => {
            let condition = get_ident(&condition);
            let true_target = get_block_fn_ident(&true_target);
            let false_target = get_block_fn_ident(&false_target);

            quote! {
                if #condition { return #true_target(state, tracer, fn_state); } else { return #false_target(state, tracer, fn_state); }
            }
        }
        StatementKind::PhiNode { .. } => quote!(todo!("phi")),
        StatementKind::Return { value } => match value {
            Some(value) => {
                let name = codegen_ident(value.name());
                quote! { return #name; }
            }
            None => {
                quote! { return; }
            }
        },
        StatementKind::Select {
            condition,
            true_value,
            false_value,
        } => {
            let condition = get_ident(&condition);
            let true_value = get_ident(&true_value);
            let false_value = get_ident(&false_value);
            quote! { if #condition { #true_value } else { #false_value } }
        }
        StatementKind::BitExtract {
            value,
            start,
            length,
        } => {
            if let Type::Bundled {
                value: bundle_value_type,
                ..
            } = &*value.typ()
            {
                let length = if let Type::Bundled { .. } = &*length.typ() {
                    let length = get_ident(&length);
                    quote!(#length.value as u32)
                } else {
                    let length = get_ident(&length);
                    quote!(#length as u32)
                };

                let value = get_ident(&value);
                let start = get_ident(&start);

                let mask_bit = match &**bundle_value_type {
                    Type::Primitive(PrimitiveType {
                        tc,
                        element_width_in_bits,
                    }) => {
                        assert!(*element_width_in_bits == 64);

                        match tc {
                            PrimitiveTypeClass::UnsignedInteger => quote!(1u64),
                            PrimitiveTypeClass::SignedInteger => quote!(1i64),
                            _ => panic!("non integer bundle value type"),
                        }
                    }
                    _ => panic!("non primitive bundle value type"),
                };

                quote! (
                    (
                        (#value >> #start) &
                        Bundle { value: ((#mask_bit).checked_shl(#length).map(|x| x - 1).unwrap_or(!0)), length: #value.length }
                    )
                )
            } else {
                let typ = codegen_type(value.typ());

                let value = get_ident(&value);
                let start = get_ident(&start);
                let length = get_ident(&length);

                // todo: pre-cast length to u32

                quote! (
                    (
                        (#value >> #start) &
                        ((1 as #typ).checked_shl(#length as u32).map(|x| x - 1).unwrap_or(!0))
                    )
                )
            }
        }
        StatementKind::BitInsert {
            original_value,
            insert_value,
            start,
            length,
        } => {
            if let Type::Bundled {
                value: bundle_value_type,
                ..
            } = &*original_value.typ()
            {
                let length = if let Type::Bundled { .. } = &*length.typ() {
                    let length = get_ident(&length);
                    quote!(#length.value as u32)
                } else {
                    let length = get_ident(&length);
                    quote!(#length as u32)
                };

                let mask_bit = match &**bundle_value_type {
                    Type::Primitive(PrimitiveType {
                        tc,
                        element_width_in_bits,
                    }) => {
                        assert!(*element_width_in_bits == 64);

                        match tc {
                            PrimitiveTypeClass::UnsignedInteger => quote!(1u64),
                            PrimitiveTypeClass::SignedInteger => quote!(1i64),
                            _ => panic!("non integer bundle value type"),
                        }
                    }
                    t => {
                        log::error!(
                            "non primitive bundle value type: {t:?} {:?}",
                            codegen_type(bundle_value_type.clone()).to_string()
                        );
                        quote!(1u64)
                    }
                };

                let original_value = get_ident(&original_value);
                let insert_value = get_ident(&insert_value);
                let start = get_ident(&start);

                quote! {
                    {
                        let mask = !Bundle { value: ((#mask_bit).checked_shl(#length).map(|x| x - 1).unwrap_or(!0)), length: #original_value.length };
                        (#original_value & mask) | (#insert_value << #start)
                    }
                }
            } else {
                let typ = codegen_type(original_value.typ());

                let original_value = get_ident(&original_value);
                let insert_value = get_ident(&insert_value);
                let start = get_ident(&start);
                let length = get_ident(&length);

                // todo: pre-cast length to u32

                quote! {
                    {
                        let mask = !(((1 as #typ).checked_shl(#length as u32).map(|x| x - 1).unwrap_or(!0)) << #start);
                        (#original_value & mask) | (#insert_value << #start)
                    }
                }
            }
        }
        StatementKind::Panic(statements) => {
            let args = statements.iter().map(get_ident);

            quote!(panic!("{:?}", (#(#args),*)))
        }
        StatementKind::ReadElement { vector, index } => {
            let index_typ = index.typ();

            let vector = get_ident(&vector);
            let index = get_ident(&index);

            if let Type::Bundled { .. } = &*index_typ {
                quote!(#vector[(#index.value) as usize])
            } else {
                quote!(#vector[(#index) as usize])
            }
        }
        StatementKind::MutateElement {
            vector,
            value,
            index,
        } => {
            let vector = get_ident(&vector);
            let index = get_ident(&index);
            let value = get_ident(&value);
            // todo: support bundle indexes
            quote! {
                {
                    let mut local = #vector.clone();
                    local[(#index) as usize] = #value;
                    local
                }
            }
        }
        StatementKind::CreateComposite { typ, fields } => {
            let typ = codegen_type(typ);
            let fields = fields
                .iter()
                .enumerate()
                .map(|(index, statement)| {
                    let field_name = codegen_member(index);
                    let value = get_ident(statement);
                    quote!(#field_name: #value,)
                })
                .collect::<TokenStream>();
            quote!(#typ { #fields })

            // struct { _0: foo, _1: bar }
        }
        StatementKind::Bundle { value, length } => {
            let value = get_ident(&value);
            let length = get_ident(&length);
            quote!(Bundle { value: #value, length: #length })
        }
        StatementKind::UnbundleValue { bundle } => {
            let bundle = get_ident(&bundle);
            quote!((#bundle.value))
        }
        StatementKind::UnbundleLength { bundle } => {
            let bundle = get_ident(&bundle);
            quote!((#bundle.length))
        }
        StatementKind::Assert { condition } => {
            let condition = get_ident(&condition);
            quote!(assert!(#condition))
        }
    };

    let msg = format!(" {} {stmt}", stmt.classify());
    if stmt.has_value() {
        let typ = codegen_type(stmt.typ());
        quote! {
            #[doc = #msg]
            let #stmt_name: #typ = #value;
        }
    } else {
        quote! {
            #[doc = #msg]
            #value;
        }
    }
}

pub fn get_functions_to_codegen(
    rudder: &Context,
    entrypoint: InternedString,
) -> Vec<InternedString> {
    let rudder_fns = rudder.get_functions();

    let mut fns = HashSet::default();
    let mut todo = vec![entrypoint];

    fn get_calls(f: &Function) -> Vec<InternedString> {
        f.entry_block()
            .iter()
            .flat_map(|b| b.statements().into_iter())
            .filter_map(|s| {
                if let StatementKind::Call { target, .. } = s.kind() {
                    Some(target.name())
                } else {
                    None
                }
            })
            .collect()
    }

    while let Some(next) = todo.pop() {
        if fns.contains(&next) {
            continue;
        }

        let Some(f) = rudder_fns.get(&next) else {
            panic!("function {next:?} called but not found in rudder context");
        };

        let next_children = get_calls(f);
        todo.extend(next_children);
        fns.insert(next);
    }

    fns.into_iter().collect()
}
