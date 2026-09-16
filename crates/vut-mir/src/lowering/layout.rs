//! Concrete ABI layouts for semantic types.
use std::collections::HashMap;

use vut_hir::TypeId;
use vut_types::{SemanticResult, Type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbiClass {
    Void,
    Scalar,
    ScalarPair,
    Aggregate,
    Indirect,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueRepr {
    Void,
    Integer,
    Float,
    Pointer,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipKind {
    None,
    RcString,
    RcBytes,
    RcList,
    RcMap,
    /// Opaque runtime task handle; releasing cancels/destroys the Vutcon.
    RcVutcon,
    /// Native async operation handle; releasing cancels/destroys it.
    Future,
    /// Owned opaque native resource handle; releasing runs the native destructor.
    Resource,
    /// Reference-counted interface/`dyn` box produced by the compiler.
    Interface,
    Aggregate,
    OpaqueManaged,
}
impl OwnershipKind {
    /// Selects the duplication strategy for this ownership class.
    #[must_use]
    pub const fn duplication(self, is_copy: bool) -> vut_memory::Duplication {
        match self {
            OwnershipKind::None | OwnershipKind::OpaqueManaged => {
                if is_copy {
                    vut_memory::Duplication::Copy
                } else {
                    vut_memory::Duplication::Opaque
                }
            }
            OwnershipKind::RcString
            | OwnershipKind::RcBytes
            | OwnershipKind::RcList
            | OwnershipKind::RcMap
            | OwnershipKind::Interface => vut_memory::Duplication::Retain,
            OwnershipKind::Aggregate => {
                if is_copy {
                    vut_memory::Duplication::Copy
                } else {
                    vut_memory::Duplication::Structural
                }
            }
            OwnershipKind::RcVutcon | OwnershipKind::Resource | OwnershipKind::Future => {
                vut_memory::Duplication::Linear
            }
        }
    }
    /// Returns `true` when values of this class are move-only.
    #[must_use]
    pub const fn is_linear(self) -> bool {
        matches!(
            self,
            OwnershipKind::RcVutcon | OwnershipKind::Resource | OwnershipKind::Future
        )
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "layout flags are consumed independently by layout, ownership, and codegen"
)]
pub struct TypeInfo {
    pub size: usize,
    pub alignment: usize,
    pub is_copy: bool,
    pub needs_drop: bool,
    pub contains_managed: bool,
    /// Whether this value (directly or through a field) owns a move-only
    /// resource, which forbids duplication of the whole value.
    pub contains_linear: bool,
    pub abi: AbiClass,
    pub repr: ValueRepr,
    pub ownership: OwnershipKind,
}
#[derive(Clone, Debug)]
pub struct LayoutTable {
    pub types: Vec<TypeInfo>,
    pub pointer_size: usize,
    pub fields: HashMap<TypeId, Vec<FieldLayout>>,
    pub arrays: HashMap<TypeId, (TypeId, usize)>,
    pub lists: HashMap<TypeId, TypeId>,
    pub maps: HashMap<TypeId, (TypeId, TypeId)>,
    pub results: HashMap<TypeId, ResultLayout>,
    /// Tagged-union layouts for enums.
    pub enums: HashMap<TypeId, EnumLayout>,
    /// Callable signatures keyed by their type, for indirect calls.
    pub callables: HashMap<TypeId, (Vec<TypeId>, TypeId)>,
    /// Types whose value is a pointer to a contiguous block (aggregate by
    /// reference): `data`, `result`, and arrays. These use the sret calling
    /// convention when returned from a function.
    pub aggregates: std::collections::HashSet<TypeId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldLayout {
    pub name: String,
    pub ty: TypeId,
    pub offset: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultLayout {
    pub ok: TypeId,
    pub err: TypeId,
    pub tag_offset: usize,
    pub ok_offset: usize,
    pub err_offset: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumLayout {
    pub tag_offset: usize,
    pub tag_size: usize,
    pub variants: Vec<VariantLayout>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantLayout {
    pub payload_offset: usize,
    pub fields: Vec<FieldLayout>,
}
impl LayoutTable {
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "primitive and aggregate ABI classification is intentionally centralized"
    )]
    pub fn from_semantics(semantics: &SemanticResult, pointer_size: usize) -> Self {
        let mut types: Vec<_> = semantics
            .types
            .iter()
            .map(|ty| match ty {
                Type::Void | Type::Null | Type::Error => TypeInfo {
                    size: 0,
                    alignment: 1,
                    is_copy: true,
                    needs_drop: false,
                    contains_managed: false,
                    contains_linear: false,
                    abi: AbiClass::Void,
                    repr: ValueRepr::Void,
                    ownership: OwnershipKind::None,
                },
                // A generic type parameter only exists while checking a generic
                // template. Monomorphization replaces it before MIR layout, so a
                // placeholder layout is never used for code generation.
                Type::Param(_) | Type::Applied(_, _) | Type::SelfType | Type::Variadic(_) => {
                    TypeInfo {
                        size: 0,
                        alignment: 1,
                        is_copy: false,
                        needs_drop: false,
                        contains_managed: false,
                        contains_linear: false,
                        abi: AbiClass::Void,
                        repr: ValueRepr::Void,
                        ownership: OwnershipKind::None,
                    }
                }
                Type::Bool => scalar(1, ValueRepr::Integer),
                Type::Numeric(name) if name == "usize" || name == "isize" => {
                    scalar(pointer_size, ValueRepr::Integer)
                }
                Type::Numeric(name) if name.ends_with('8') => scalar(1, ValueRepr::Integer),
                Type::Numeric(name) if name.ends_with("16") => scalar(2, ValueRepr::Integer),
                Type::Numeric(name) if name.ends_with("32") => scalar(
                    4,
                    if name.starts_with('f') {
                        ValueRepr::Float
                    } else {
                        ValueRepr::Integer
                    },
                ),
                Type::Float => scalar(8, ValueRepr::Float),
                Type::Int | Type::Numeric(_) => scalar(8, ValueRepr::Integer),
                Type::Pointer(_) | Type::FunctionPointer { .. } => {
                    scalar(pointer_size, ValueRepr::Pointer)
                }
                Type::Str => managed(pointer_size, pointer_size, OwnershipKind::RcString),
                Type::List(_) => managed(pointer_size, pointer_size, OwnershipKind::RcList),
                Type::Map(_, _) => managed(pointer_size, pointer_size, OwnershipKind::RcMap),
                Type::Bytes => managed(pointer_size, pointer_size, OwnershipKind::RcBytes),
                Type::Vutcon(_) => managed(pointer_size, pointer_size, OwnershipKind::RcVutcon),
                Type::Future(_) => managed(pointer_size, pointer_size, OwnershipKind::Future),
                Type::Resource(_) => managed(pointer_size, pointer_size, OwnershipKind::Resource),
                Type::Array(_, _) | Type::Result(_, _) | Type::Enum(_) => TypeInfo {
                    size: 0,
                    alignment: 1,
                    is_copy: false,
                    needs_drop: false,
                    contains_managed: false,
                    contains_linear: false,
                    abi: AbiClass::Aggregate,
                    repr: ValueRepr::Pointer,
                    ownership: OwnershipKind::Aggregate,
                },
                Type::Dyn | Type::Interface(_) => {
                    managed(pointer_size, pointer_size, OwnershipKind::Interface)
                }
                Type::Optional(inner) => {
                    let nested = &semantics.types[inner.0];
                    // An optional managed handle is a nullable handle of the
                    // inner type, so it shares the inner ownership kind. The
                    // runtime retain/release entry points are null-safe, which
                    // gives `null` the correct no-op behavior.
                    match nested {
                        Type::Str => managed(pointer_size, pointer_size, OwnershipKind::RcString),
                        Type::Bytes => managed(pointer_size, pointer_size, OwnershipKind::RcBytes),
                        Type::List(_) => managed(pointer_size, pointer_size, OwnershipKind::RcList),
                        Type::Map(_, _) => {
                            managed(pointer_size, pointer_size, OwnershipKind::RcMap)
                        }
                        Type::Vutcon(_) => {
                            managed(pointer_size, pointer_size, OwnershipKind::RcVutcon)
                        }
                        Type::Future(_) => {
                            managed(pointer_size, pointer_size, OwnershipKind::Future)
                        }
                        Type::Resource(_) => {
                            managed(pointer_size, pointer_size, OwnershipKind::Resource)
                        }
                        Type::Interface(_) | Type::Dyn => {
                            managed(pointer_size, pointer_size, OwnershipKind::Interface)
                        }
                        _ => TypeInfo {
                            size: 16,
                            alignment: 8,
                            is_copy: false,
                            needs_drop: false,
                            contains_managed: false,
                            contains_linear: false,
                            abi: AbiClass::Aggregate,
                            repr: ValueRepr::Pointer,
                            ownership: OwnershipKind::None,
                        },
                    }
                }
                Type::Data(symbol) if semantics.attributes.data_repr.contains_key(symbol) => {
                    TypeInfo {
                        size: 0,
                        alignment: 1,
                        is_copy: true,
                        needs_drop: false,
                        contains_managed: false,
                        contains_linear: false,
                        abi: AbiClass::Aggregate,
                        repr: ValueRepr::Pointer,
                        ownership: OwnershipKind::None,
                    }
                }
                Type::Data(symbol) if semantics.opaque_data.contains(symbol) => TypeInfo {
                    size: 0,
                    alignment: 1,
                    is_copy: true,
                    needs_drop: false,
                    contains_managed: false,
                    contains_linear: false,
                    abi: AbiClass::Void,
                    repr: ValueRepr::Void,
                    ownership: OwnershipKind::None,
                },
                Type::Data(_) => TypeInfo {
                    size: pointer_size,
                    alignment: pointer_size,
                    is_copy: false,
                    needs_drop: true,
                    contains_managed: true,
                    contains_linear: false,
                    abi: AbiClass::Indirect,
                    repr: ValueRepr::Pointer,
                    ownership: OwnershipKind::Aggregate,
                },
                Type::Function(_) | Type::Callable { .. } => TypeInfo {
                    size: pointer_size,
                    alignment: pointer_size,
                    is_copy: true,
                    needs_drop: false,
                    contains_managed: false,
                    contains_linear: false,
                    abi: AbiClass::Scalar,
                    repr: ValueRepr::Pointer,
                    ownership: OwnershipKind::None,
                },
                Type::Range(_) => TypeInfo {
                    size: 16,
                    alignment: 8,
                    is_copy: true,
                    needs_drop: false,
                    contains_managed: false,
                    contains_linear: false,
                    abi: AbiClass::ScalarPair,
                    repr: ValueRepr::Pointer,
                    ownership: OwnershipKind::None,
                },
            })
            .collect();
        let lists = semantics
            .types
            .iter()
            .enumerate()
            .filter_map(|(type_index, ty)| match ty {
                Type::List(element_id) => Some((TypeId(type_index), *element_id)),
                _ => None,
            })
            .collect();
        let maps = semantics
            .types
            .iter()
            .enumerate()
            .filter_map(|(type_index, ty)| match ty {
                Type::Map(key, value) => Some((TypeId(type_index), (*key, *value))),
                _ => None,
            })
            .collect();
        // Aggregate layouts (arrays, results, data) may reference other
        // aggregates. Compute them in dependency order so a nested type has a
        // final size/alignment before its parent is laid out.
        let mut arrays = HashMap::new();
        let mut results = HashMap::new();
        let mut enums = HashMap::new();
        let mut fields = HashMap::new();
        let is_aggregate = |ty: &Type| {
            matches!(
                ty,
                Type::Array(..) | Type::Result(..) | Type::Data(_) | Type::Enum(_)
            )
        };
        let mut resolved: Vec<bool> = semantics
            .types
            .iter()
            .map(|ty| {
                let opaque =
                    matches!(ty, Type::Data(symbol) if semantics.opaque_data.contains(symbol));
                opaque || !is_aggregate(ty)
            })
            .collect();
        let mut pending = true;
        while pending {
            pending = false;
            for (type_index, ty) in semantics.types.iter().enumerate() {
                if resolved[type_index] {
                    continue;
                }
                match ty {
                    Type::Array(element_id, length) => {
                        if !resolved[element_id.0] {
                            continue;
                        }
                        let element = types[element_id.0];
                        arrays.insert(TypeId(type_index), (*element_id, *length));
                        let size = element.size.saturating_mul(*length);
                        types[type_index] = TypeInfo {
                            size,
                            alignment: element.alignment,
                            is_copy: element.is_copy,
                            needs_drop: element.needs_drop,
                            contains_managed: element.contains_managed,
                            contains_linear: element.contains_linear,
                            abi: abi_class(size, pointer_size),
                            repr: ValueRepr::Pointer,
                            ownership: OwnershipKind::Aggregate,
                        };
                        resolved[type_index] = true;
                        pending = true;
                    }
                    Type::Result(ok, err) => {
                        if !resolved[ok.0] || !resolved[err.0] {
                            continue;
                        }
                        let ok_info = types[ok.0];
                        let err_info = types[err.0];
                        let mut offset = pointer_size;
                        let alignment = pointer_size.max(ok_info.alignment).max(err_info.alignment);
                        offset = align_up(offset, ok_info.alignment);
                        let ok_offset = offset;
                        offset += ok_info.size;
                        offset = align_up(offset, err_info.alignment);
                        let err_offset = offset;
                        offset += err_info.size;
                        let size = align_up(offset, alignment);
                        types[type_index] = TypeInfo {
                            size,
                            alignment,
                            is_copy: ok_info.is_copy && err_info.is_copy,
                            needs_drop: ok_info.needs_drop || err_info.needs_drop,
                            contains_managed: ok_info.contains_managed || err_info.contains_managed,
                            contains_linear: ok_info.contains_linear || err_info.contains_linear,
                            abi: abi_class(size, pointer_size),
                            repr: ValueRepr::Pointer,
                            ownership: OwnershipKind::Aggregate,
                        };
                        results.insert(
                            TypeId(type_index),
                            ResultLayout {
                                ok: *ok,
                                err: *err,
                                tag_offset: 0,
                                ok_offset,
                                err_offset,
                            },
                        );
                        resolved[type_index] = true;
                        pending = true;
                    }
                    Type::Enum(symbol) => {
                        let Some(variants) = semantics.enum_variants.get(symbol) else {
                            resolved[type_index] = true;
                            pending = true;
                            continue;
                        };
                        if variants
                            .iter()
                            .any(|variant| variant.fields.iter().any(|field| !resolved[field.ty.0]))
                        {
                            continue;
                        }
                        let tag_size = if variants.len() <= 256 {
                            1
                        } else if variants.len() <= 65_536 {
                            2
                        } else {
                            4
                        };
                        let max_align = variants
                            .iter()
                            .flat_map(|variant| variant.fields.iter())
                            .map(|field| types[field.ty.0].alignment)
                            .max()
                            .unwrap_or(1)
                            .max(1);
                        let payload_offset = align_up(tag_size, max_align);
                        let mut variant_layouts = Vec::with_capacity(variants.len());
                        let mut payload_size = 0;
                        let mut is_copy = true;
                        let mut needs_drop = false;
                        let mut contains_managed = false;
                        let mut contains_linear = false;
                        for variant in variants {
                            let mut offset = payload_offset;
                            let mut field_layouts = Vec::with_capacity(variant.fields.len());
                            for field in &variant.fields {
                                let info = types[field.ty.0];
                                offset = align_up(offset, info.alignment);
                                field_layouts.push(FieldLayout {
                                    name: field.name.clone(),
                                    ty: field.ty,
                                    offset,
                                });
                                offset += info.size;
                                is_copy &= info.is_copy;
                                needs_drop |= info.needs_drop;
                                contains_managed |= info.contains_managed;
                                contains_linear |= info.contains_linear;
                            }
                            payload_size = payload_size.max(offset.saturating_sub(payload_offset));
                            variant_layouts.push(VariantLayout {
                                payload_offset,
                                fields: field_layouts,
                            });
                        }
                        let alignment = max_align.max(tag_size);
                        let size = align_up(payload_offset + payload_size, alignment);
                        types[type_index] = TypeInfo {
                            size,
                            alignment,
                            is_copy,
                            needs_drop,
                            contains_managed,
                            contains_linear,
                            abi: abi_class(size, pointer_size),
                            repr: ValueRepr::Pointer,
                            ownership: OwnershipKind::Aggregate,
                        };
                        enums.insert(
                            TypeId(type_index),
                            EnumLayout {
                                tag_offset: 0,
                                tag_size,
                                variants: variant_layouts,
                            },
                        );
                        resolved[type_index] = true;
                        pending = true;
                    }
                    Type::Data(symbol) => {
                        let Some(data_fields) = semantics.data_fields.get(symbol) else {
                            resolved[type_index] = true;
                            pending = true;
                            continue;
                        };
                        if data_fields.iter().any(|field| !resolved[field.ty.0]) {
                            continue;
                        }
                        let mut offset = 0;
                        let mut alignment = 1;
                        let mut layouts = Vec::with_capacity(data_fields.len());
                        let mut is_copy = true;
                        let mut needs_drop = false;
                        let mut contains_managed = false;
                        let mut contains_linear = false;
                        for field in data_fields {
                            let info = types[field.ty.0];
                            offset = align_up(offset, info.alignment);
                            layouts.push(FieldLayout {
                                name: field.name.clone(),
                                ty: field.ty,
                                offset,
                            });
                            offset += info.size;
                            alignment = alignment.max(info.alignment);
                            is_copy &= info.is_copy;
                            needs_drop |= info.needs_drop;
                            contains_managed |= info.contains_managed;
                            contains_linear |= info.contains_linear;
                        }
                        let size = align_up(offset, alignment);
                        let ffi_repr = semantics.attributes.data_repr.contains_key(symbol);
                        types[type_index] = TypeInfo {
                            size,
                            alignment,
                            is_copy,
                            needs_drop: if ffi_repr { false } else { needs_drop },
                            contains_managed: if ffi_repr { false } else { contains_managed },
                            contains_linear: if ffi_repr { false } else { contains_linear },
                            abi: abi_class(size, pointer_size),
                            repr: ValueRepr::Pointer,
                            ownership: if ffi_repr {
                                OwnershipKind::None
                            } else {
                                OwnershipKind::Aggregate
                            },
                        };
                        fields.insert(TypeId(type_index), layouts);
                        resolved[type_index] = true;
                        pending = true;
                    }
                    _ => {}
                }
            }
        }
        // The backend ABI flattens a receiver into a hidden leading parameter;
        // the semantic `Type::Callable::receiver` stays distinct.
        let callable_abi_parameters = |receiver: Option<TypeId>,
                                       parameters: &[TypeId]|
         -> Vec<TypeId> {
            let mut abi = Vec::with_capacity(parameters.len() + usize::from(receiver.is_some()));
            if let Some(receiver) = receiver {
                abi.push(receiver);
            }
            abi.extend_from_slice(parameters);
            abi
        };
        let mut callables = HashMap::new();
        for (type_index, ty) in semantics.types.iter().enumerate() {
            match ty {
                Type::Callable {
                    receiver,
                    parameters,
                    result,
                } => {
                    callables.insert(
                        TypeId(type_index),
                        (callable_abi_parameters(*receiver, parameters), *result),
                    );
                }
                Type::FunctionPointer {
                    parameters, result, ..
                } => {
                    callables.insert(TypeId(type_index), (parameters.clone(), *result));
                }
                Type::Function(symbol) => {
                    if let Some(signature) = semantics.function_signatures.get(symbol) {
                        callables.insert(
                            TypeId(type_index),
                            (
                                callable_abi_parameters(signature.receiver, &signature.parameters),
                                signature.result,
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        let aggregates = semantics
            .types
            .iter()
            .enumerate()
            .filter_map(|(type_index, ty)| {
                matches!(
                    ty,
                    Type::Data(_) | Type::Result(_, _) | Type::Array(_, _) | Type::Enum(_)
                )
                .then_some(TypeId(type_index))
            })
            .collect();
        Self {
            types,
            pointer_size,
            fields,
            arrays,
            lists,
            maps,
            results,
            enums,
            callables,
            aggregates,
        }
    }

    /// Returns `true` when values of `ty` are aggregate-by-reference blocks that
    /// must be returned through a caller-provided destination (sret).
    #[must_use]
    pub fn is_aggregate(&self, ty: TypeId) -> bool {
        self.aggregates.contains(&ty)
    }

    /// Storage size of one `ty` value inside a managed collection.
    ///
    /// Elements are stored inline, including aggregates; a collection slot owns
    /// the element bytes and aggregate elements are referenced by the address
    /// of that slot.
    #[must_use]
    pub fn element_storage_size(&self, ty: TypeId) -> usize {
        self.types[ty.0].size
    }

    /// Alignment of one `ty` value inside a managed collection.
    #[must_use]
    pub fn element_storage_align(&self, ty: TypeId) -> usize {
        self.types[ty.0].alignment.max(1)
    }

    #[must_use]
    pub fn field(&self, ty: TypeId, name: &str) -> Option<&FieldLayout> {
        self.fields
            .get(&ty)?
            .iter()
            .find(|field| field.name == name)
    }
}
fn align_up(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}
fn abi_class(size: usize, pointer_size: usize) -> AbiClass {
    if size <= pointer_size {
        AbiClass::Scalar
    } else if size <= pointer_size * 2 {
        AbiClass::ScalarPair
    } else {
        AbiClass::Indirect
    }
}
fn scalar(size: usize, repr: ValueRepr) -> TypeInfo {
    TypeInfo {
        size,
        alignment: size,
        is_copy: true,
        needs_drop: false,
        contains_managed: false,
        contains_linear: false,
        abi: AbiClass::Scalar,
        repr,
        ownership: OwnershipKind::None,
    }
}
fn managed(size: usize, alignment: usize, ownership: OwnershipKind) -> TypeInfo {
    TypeInfo {
        size,
        alignment,
        is_copy: false,
        needs_drop: true,
        contains_managed: true,
        contains_linear: ownership.is_linear(),
        abi: AbiClass::Aggregate,
        repr: ValueRepr::Pointer,
        ownership,
    }
}
pub(super) fn is_unsigned_type(ty: &Type) -> bool {
    matches!(ty, Type::Numeric(name) if matches!(name.as_str(), "u8" | "u16" | "u32" | "u64"))
}
