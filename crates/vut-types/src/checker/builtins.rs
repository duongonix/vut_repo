//! Builtin method resolution and construction helpers.
use super::context::Context;
#[allow(clippy::wildcard_imports)]
use super::*;

impl Analyzer<'_> {
    pub(super) fn try_bytes_constructor(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let bytes = self.intern(Type::Bytes);
        match callee {
            Expr::Name(name) if name.text == "bytes" => {
                if !arguments.is_empty() {
                    self.error(
                        codes::E1003,
                        "invalid bytes constructor",
                        span,
                        "`bytes()` takes no arguments; use `list(u8).to_bytes()` for conversion",
                    );
                    return Some(self.intern(Type::Error));
                }
                self.builtin_calls.insert(span, BuiltinFunction::BytesNew);
                Some(bytes)
            }
            Expr::Member { object, member, .. }
                if member.text == "from_list"
                    && matches!(object.as_ref(), Expr::Name(name) if name.text == "bytes") =>
            {
                if arguments.len() != 1 || arguments[0].name.is_some() {
                    self.error(
                        codes::E1003,
                        "invalid bytes conversion",
                        span,
                        "`bytes.from_list` takes one positional `list(u8)` argument",
                    );
                    return Some(self.intern(Type::Error));
                }
                let actual = self.expr(module, &arguments[0].value, context, None);
                let u8_ty = self.intern(Type::Numeric("u8".into()));
                let expected = self.intern(Type::List(u8_ty));
                self.compatible(
                    actual,
                    expected,
                    arguments[0].value.span(),
                    codes::E1003,
                    "bytes conversion requires list(u8)",
                );
                self.builtin_calls
                    .insert(span, BuiltinFunction::BytesFromList);
                Some(bytes)
            }
            _ => None,
        }
    }

    pub(super) fn try_builtin_call(
        &mut self,
        module: &Module,
        callee: &Expr,
        arguments: &[vut_ast::Argument],
        span: Span,
        context: &mut Context,
    ) -> Option<TypeId> {
        let Expr::Member { object, member, .. } = callee else {
            return None;
        };
        let owner = self.expr(module, object, context, None);
        let Some((builtin, signature)) = self.builtin_method(owner, &member.text) else {
            if matches!(self.types[owner.0], Type::Array(_, _))
                && matches!(
                    member.text.as_str(),
                    "push" | "pop" | "insert" | "remove" | "clear" | "reserve" | "capacity"
                )
            {
                self.error(
                    codes::E2005,
                    "method is not available for arrays",
                    member.span,
                    "use `list(T)` / `@(...)` when a growable collection is required",
                );
                return Some(self.intern(Type::Error));
            }
            return None;
        };
        if let Type::Array(_, length) = self.types[owner.0]
            && matches!(
                builtin,
                BuiltinFunction::ArrayAt | BuiltinFunction::ArraySet
            )
            && let Some(vut_ast::Argument {
                value: Expr::Integer { text, span },
                ..
            }) = arguments.first()
            && text
                .replace('_', "")
                .parse::<usize>()
                .is_ok_and(|index| index >= length)
        {
            self.error(
                codes::E1003,
                "array index out of bounds",
                *span,
                &format!("index must be less than array length {length}"),
            );
        }
        self.arguments(module, &signature, arguments, span, context, false);
        self.builtin_calls.insert(span, builtin);
        Some(signature.result)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "builtin method dispatch table is intentionally centralized"
    )]
    pub(super) fn builtin_method(
        &mut self,
        owner: TypeId,
        name: &str,
    ) -> Option<(BuiltinFunction, Signature)> {
        let int = self.intern(Type::Int);
        let bool_ty = self.intern(Type::Bool);
        let string = self.intern(Type::Str);
        let bytes = self.intern(Type::Bytes);
        let void = self.intern(Type::Void);
        let no_args = |result| Signature {
            parameters: Vec::new(),
            result,
        };
        match (&self.types[owner.0], name) {
            (Type::Str, "byte_len") => Some((BuiltinFunction::StringByteLen, no_args(int))),
            (Type::Str, "char_len") => Some((BuiltinFunction::StringCharLen, no_args(int))),
            (Type::Str, "is_empty") => Some((BuiltinFunction::StringIsEmpty, no_args(bool_ty))),
            (Type::Str, "contains") => Some((
                BuiltinFunction::StringContains,
                Signature {
                    parameters: vec![("pattern".into(), string)],
                    result: bool_ty,
                },
            )),
            (Type::Str, "starts_with") => Some((
                BuiltinFunction::StringStartsWith,
                Signature {
                    parameters: vec![("prefix".into(), string)],
                    result: bool_ty,
                },
            )),
            (Type::Str, "ends_with") => Some((
                BuiltinFunction::StringEndsWith,
                Signature {
                    parameters: vec![("suffix".into(), string)],
                    result: bool_ty,
                },
            )),
            (Type::Str, "trim") => Some((BuiltinFunction::StringTrim, no_args(string))),
            (Type::Str, "trim_start") => Some((BuiltinFunction::StringTrimStart, no_args(string))),
            (Type::Str, "trim_end") => Some((BuiltinFunction::StringTrimEnd, no_args(string))),
            (Type::Str, "to_lower") => Some((BuiltinFunction::StringToLower, no_args(string))),
            (Type::Str, "to_upper") => Some((BuiltinFunction::StringToUpper, no_args(string))),
            (Type::Str, "replace") => Some((
                BuiltinFunction::StringReplace,
                Signature {
                    parameters: vec![("from".into(), string), ("to".into(), string)],
                    result: string,
                },
            )),
            (Type::Str, "to_bytes") => Some((BuiltinFunction::StringToBytes, no_args(bytes))),
            (Type::Str, "to_i64") => Some((BuiltinFunction::StringToI64, no_args(int))),
            (Type::Str, "to_f64") => Some((
                BuiltinFunction::StringToF64,
                no_args(self.intern(Type::Float)),
            )),
            (Type::Str, "find") => Some((
                BuiltinFunction::StringFind,
                Signature {
                    parameters: vec![("needle".into(), string)],
                    result: int,
                },
            )),
            (Type::Str, "split") => Some((
                BuiltinFunction::StringSplit,
                Signature {
                    parameters: vec![("separator".into(), string)],
                    result: self.intern(Type::List(string)),
                },
            )),
            (Type::Str, "substring") => Some((
                BuiltinFunction::StringSubstring,
                Signature {
                    parameters: vec![("start".into(), int), ("end".into(), int)],
                    result: string,
                },
            )),
            (Type::Bytes, _) => self.builtin_bytes_method(name),
            (Type::Variadic(_), "len") => Some((BuiltinFunction::VariadicLen, no_args(int))),
            (Type::Variadic(element), "at") => Some((
                BuiltinFunction::VariadicAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: *element,
                },
            )),
            (Type::List(element), _) => self.builtin_list_method(*element, name),
            (Type::Map(key, value), _) => self.builtin_map_method(*key, *value, name),
            (Type::Array(_, _), "len") => Some((BuiltinFunction::ArrayLen, no_args(int))),
            (Type::Array(element, _), "at") => Some((
                BuiltinFunction::ArrayAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: *element,
                },
            )),
            (Type::Array(element, _), "set") => Some((
                BuiltinFunction::ArraySet,
                Signature {
                    parameters: vec![("index".into(), int), ("value".into(), *element)],
                    result: void,
                },
            )),
            (Type::Array(element, _), "first") => {
                Some((BuiltinFunction::ArrayFirst, no_args(*element)))
            }
            (Type::Array(element, _), "last") => {
                Some((BuiltinFunction::ArrayLast, no_args(*element)))
            }
            (Type::Array(element, _), "fill") => Some((
                BuiltinFunction::ArrayFill,
                Signature {
                    parameters: vec![("value".into(), *element)],
                    result: void,
                },
            )),
            (Type::Int | Type::Float | Type::Numeric(_), "to_str") => {
                Some((BuiltinFunction::NumericToStr, no_args(string)))
            }
            (Type::Int, "to_char") => Some((BuiltinFunction::IntToChar, no_args(string))),
            (Type::Int, "to_float") => Some((
                BuiltinFunction::IntToFloat,
                no_args(self.intern(Type::Float)),
            )),
            _ => None,
        }
    }

    pub(super) fn builtin_list_method(
        &mut self,
        element: TypeId,
        name: &str,
    ) -> Option<(BuiltinFunction, Signature)> {
        let int = self.intern(Type::Int);
        let bool_ty = self.intern(Type::Bool);
        let void = self.intern(Type::Void);
        let no_args = |result| Signature {
            parameters: Vec::new(),
            result,
        };
        match name {
            "len" => Some((BuiltinFunction::ListLen, no_args(int))),
            "is_empty" => Some((BuiltinFunction::ListIsEmpty, no_args(bool_ty))),
            "capacity" => Some((BuiltinFunction::ListCapacity, no_args(int))),
            "reserve" => Some((
                BuiltinFunction::ListReserve,
                Signature {
                    parameters: vec![("capacity".into(), int)],
                    result: void,
                },
            )),
            "push" => Some((
                BuiltinFunction::ListPush,
                Signature {
                    parameters: vec![("value".into(), element)],
                    result: void,
                },
            )),
            "at" => Some((
                BuiltinFunction::ListAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: element,
                },
            )),
            "set" => Some((
                BuiltinFunction::ListSet,
                Signature {
                    parameters: vec![("index".into(), int), ("value".into(), element)],
                    result: void,
                },
            )),
            "insert" => Some((
                BuiltinFunction::ListInsert,
                Signature {
                    parameters: vec![("index".into(), int), ("value".into(), element)],
                    result: void,
                },
            )),
            "remove" => Some((
                BuiltinFunction::ListRemove,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: element,
                },
            )),
            "clear" => Some((BuiltinFunction::ListClear, no_args(void))),
            "slice" => Some((
                BuiltinFunction::ListSlice,
                Signature {
                    parameters: vec![("start".into(), int), ("end".into(), int)],
                    result: self.intern(Type::List(element)),
                },
            )),
            "contains" => Some((
                BuiltinFunction::ListContains,
                Signature {
                    parameters: vec![("value".into(), element)],
                    result: bool_ty,
                },
            )),
            "to_bytes" if matches!(&self.types[element.0], Type::Numeric(name) if name == "u8") => {
                Some((
                    BuiltinFunction::BytesFromList,
                    no_args(self.intern(Type::Bytes)),
                ))
            }
            _ => None,
        }
    }

    pub(super) fn builtin_bytes_method(
        &mut self,
        name: &str,
    ) -> Option<(BuiltinFunction, Signature)> {
        let int = self.intern(Type::Int);
        let bool_ty = self.intern(Type::Bool);
        let void = self.intern(Type::Void);
        let u8_ty = self.intern(Type::Numeric("u8".into()));
        let bytes = self.intern(Type::Bytes);
        let string = self.intern(Type::Str);
        let utf8_error = self.intern(Type::Data(self.builtin_utf8_error));
        let no_args = |result| Signature {
            parameters: Vec::new(),
            result,
        };
        match name {
            "len" => Some((BuiltinFunction::BytesLen, no_args(int))),
            "is_empty" => Some((BuiltinFunction::BytesIsEmpty, no_args(bool_ty))),
            "capacity" => Some((BuiltinFunction::BytesCapacity, no_args(int))),
            "reserve" => Some((
                BuiltinFunction::BytesReserve,
                Signature {
                    parameters: vec![("capacity".into(), int)],
                    result: void,
                },
            )),
            "at" => Some((
                BuiltinFunction::BytesAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: u8_ty,
                },
            )),
            "set" => Some((
                BuiltinFunction::BytesSet,
                Signature {
                    parameters: vec![("index".into(), int), ("value".into(), u8_ty)],
                    result: void,
                },
            )),
            "first" => Some((BuiltinFunction::BytesFirst, no_args(u8_ty))),
            "last" => Some((BuiltinFunction::BytesLast, no_args(u8_ty))),
            "slice" => Some((
                BuiltinFunction::BytesSlice,
                Signature {
                    parameters: vec![("start".into(), int), ("end".into(), int)],
                    result: bytes,
                },
            )),
            "read_i32" => Some((
                BuiltinFunction::BytesReadI32,
                Signature {
                    parameters: vec![("offset".into(), int)],
                    result: int,
                },
            )),
            "read_i64" => Some((
                BuiltinFunction::BytesReadI64,
                Signature {
                    parameters: vec![("offset".into(), int)],
                    result: int,
                },
            )),
            "byte_at" => Some((
                BuiltinFunction::BytesByteAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: int,
                },
            )),
            "clear" => Some((BuiltinFunction::BytesClear, no_args(void))),
            "to_list" => Some((
                BuiltinFunction::BytesToList,
                no_args(self.intern(Type::List(u8_ty))),
            )),
            "to_str" => Some((
                BuiltinFunction::BytesToStr,
                no_args(self.intern(Type::Result(string, utf8_error))),
            )),
            _ => None,
        }
    }

    pub(super) fn builtin_map_method(
        &mut self,
        key: TypeId,
        value: TypeId,
        name: &str,
    ) -> Option<(BuiltinFunction, Signature)> {
        let int = self.intern(Type::Int);
        let bool_ty = self.intern(Type::Bool);
        let void = self.intern(Type::Void);
        let no_args = |result| Signature {
            parameters: Vec::new(),
            result,
        };
        match name {
            "len" => Some((BuiltinFunction::MapLen, no_args(int))),
            "is_empty" => Some((BuiltinFunction::MapIsEmpty, no_args(bool_ty))),
            "capacity" => Some((BuiltinFunction::MapCapacity, no_args(int))),
            "reserve" => Some((
                BuiltinFunction::MapReserve,
                Signature {
                    parameters: vec![("capacity".into(), int)],
                    result: void,
                },
            )),
            "get" => Some((
                BuiltinFunction::MapGet,
                Signature {
                    parameters: vec![("key".into(), key)],
                    result: value,
                },
            )),
            "set" => Some((
                BuiltinFunction::MapSet,
                Signature {
                    parameters: vec![("key".into(), key), ("value".into(), value)],
                    result: void,
                },
            )),
            "contains_key" => Some((
                BuiltinFunction::MapContainsKey,
                Signature {
                    parameters: vec![("key".into(), key)],
                    result: bool_ty,
                },
            )),
            "remove" => Some((
                BuiltinFunction::MapRemove,
                Signature {
                    parameters: vec![("key".into(), key)],
                    result: value,
                },
            )),
            "clear" => Some((BuiltinFunction::MapClear, no_args(void))),
            "keys" => {
                if matches!(self.types[key.0], Type::Str) {
                    let string = self.intern(Type::Str);
                    Some((
                        BuiltinFunction::MapKeys,
                        no_args(self.intern(Type::List(string))),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
