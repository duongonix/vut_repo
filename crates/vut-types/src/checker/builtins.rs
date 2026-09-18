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
            Expr::Member { object, member, .. }
                if member.text == "from_hex"
                    && matches!(object.as_ref(), Expr::Name(name) if name.text == "bytes") =>
            {
                if arguments.len() != 1 || arguments[0].name.is_some() {
                    self.error(
                        codes::E1003,
                        "invalid bytes conversion",
                        span,
                        "`bytes.from_hex` takes one positional `str` argument",
                    );
                    return Some(self.intern(Type::Error));
                }
                let actual = self.expr(module, &arguments[0].value, context, None);
                let string = self.intern(Type::Str);
                self.compatible(
                    actual,
                    string,
                    arguments[0].value.span(),
                    codes::E1003,
                    "bytes.from_hex requires a `str`",
                );
                let hex_error = self.intern(Type::Data(self.builtin_hex_error));
                self.builtin_calls
                    .insert(span, BuiltinFunction::BytesFromHex);
                Some(self.intern(Type::Result(bytes, hex_error)))
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
        if let Type::List(element) = self.types[owner.0]
            && let Some(result) =
                self.try_list_higher_order(module, element, &member.text, arguments, span, context)
        {
            return Some(result);
        }
        let Some((builtin, signature)) = self.builtin_method(owner, &member.text) else {
            if let Type::List(element) = self.types[owner.0]
                && member.text == "join"
                && !matches!(self.types[element.0], Type::Str)
            {
                self.error(
                    codes::E1028,
                    "join is only available for list(str)",
                    member.span,
                    "convert elements to strings first, for example with `map`",
                );
                return Some(self.intern(Type::Error));
            }
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
            (Type::Str, "to_i64" | "to_int") => Some((BuiltinFunction::StringToI64, no_args(int))),
            (Type::Str, "to_f64" | "to_float") => Some((
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
            (Type::Str, "lines") => Some((
                BuiltinFunction::StringLines,
                no_args(self.intern(Type::List(string))),
            )),
            (Type::Str, "split_whitespace") => Some((
                BuiltinFunction::StringSplitWhitespace,
                no_args(self.intern(Type::List(string))),
            )),
            (Type::Str, "chars") => Some((
                BuiltinFunction::StringChars,
                no_args(self.intern(Type::List(string))),
            )),
            (Type::Str, "char_at") => Some((
                BuiltinFunction::StringCharAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: string,
                },
            )),
            (Type::Str, "repeat") => Some((
                BuiltinFunction::StringRepeat,
                Signature {
                    parameters: vec![("count".into(), int)],
                    result: string,
                },
            )),
            (Type::Str, "pad_left") => Some((
                BuiltinFunction::StringPadLeft,
                Signature {
                    parameters: vec![("width".into(), int), ("fill".into(), string)],
                    result: string,
                },
            )),
            (Type::Str, "pad_right") => Some((
                BuiltinFunction::StringPadRight,
                Signature {
                    parameters: vec![("width".into(), int), ("fill".into(), string)],
                    result: string,
                },
            )),
            (Type::Str, "strip_prefix") => Some((
                BuiltinFunction::StringStripPrefix,
                Signature {
                    parameters: vec![("prefix".into(), string)],
                    result: string,
                },
            )),
            (Type::Str, "strip_suffix") => Some((
                BuiltinFunction::StringStripSuffix,
                Signature {
                    parameters: vec![("suffix".into(), string)],
                    result: string,
                },
            )),
            (Type::Str, "rfind") => Some((
                BuiltinFunction::StringRfind,
                Signature {
                    parameters: vec![("needle".into(), string)],
                    result: int,
                },
            )),
            (Type::Str, "compare") => Some((
                BuiltinFunction::StringCompare,
                Signature {
                    parameters: vec![("other".into(), string)],
                    result: int,
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
            (Type::Array(element, _), "to_list") => Some((
                BuiltinFunction::ArrayToList,
                no_args(self.intern(Type::List(*element))),
            )),
            (Type::Array(element, _), "contains") => Some((
                BuiltinFunction::ArrayContains,
                Signature {
                    parameters: vec![("value".into(), *element)],
                    result: bool_ty,
                },
            )),
            (Type::Array(_, _), "reverse") => Some((BuiltinFunction::ArrayReverse, no_args(void))),
            (Type::Array(element, _), "sort")
                if matches!(
                    &self.types[element.0],
                    Type::Int | Type::Float | Type::Numeric(_) | Type::Str | Type::Bool
                ) =>
            {
                Some((BuiltinFunction::ArraySort, no_args(void)))
            }
            (Type::Int | Type::Float | Type::Numeric(_), "to_str") => {
                Some((BuiltinFunction::NumericToStr, no_args(string)))
            }
            (Type::Int, "to_char") => Some((BuiltinFunction::IntToChar, no_args(string))),
            (Type::Int, "to_float") => Some((
                BuiltinFunction::IntToFloat,
                no_args(self.intern(Type::Float)),
            )),
            (Type::Bool, "to_str") => Some((BuiltinFunction::BoolToStr, no_args(string))),
            (Type::Result(_, _), "is_ok") => Some((BuiltinFunction::ResultIsOk, no_args(bool_ty))),
            (Type::Result(_, _), "is_err") => {
                Some((BuiltinFunction::ResultIsErr, no_args(bool_ty)))
            }
            (Type::Result(ok, _), "unwrap_or") => Some((
                BuiltinFunction::ResultUnwrapOr,
                Signature {
                    parameters: vec![("default".into(), *ok)],
                    result: *ok,
                },
            )),
            _ => self.numeric_method(owner, name),
        }
    }

    /// Numeric scalar methods (`abs`, `pow`, `min`, `max`, `clamp`, float
    /// rounding) dispatched from the shared numeric table.
    fn numeric_method(
        &mut self,
        owner: TypeId,
        name: &str,
    ) -> Option<(BuiltinFunction, Signature)> {
        let is_float = match &self.types[owner.0] {
            Type::Float => true,
            Type::Int => false,
            Type::Numeric(name) => {
                if name == "f64" {
                    true
                } else if name.starts_with('f') {
                    // `f32` math is not supported yet.
                    return None;
                } else {
                    false
                }
            }
            _ => return None,
        };
        let int = self.intern(Type::Int);
        let float = self.intern(Type::Float);
        let bool_ty = self.intern(Type::Bool);
        let no_args = |result| Signature {
            parameters: Vec::new(),
            result,
        };
        let one = |ty, result| Signature {
            parameters: vec![("value".into(), ty)],
            result,
        };
        match (is_float, name) {
            (_, "abs") => Some((
                if is_float {
                    BuiltinFunction::FloatAbs
                } else {
                    BuiltinFunction::IntAbs
                },
                no_args(owner),
            )),
            (_, "min") => Some((
                if is_float {
                    BuiltinFunction::FloatMin
                } else {
                    BuiltinFunction::IntMin
                },
                one(owner, owner),
            )),
            (_, "max") => Some((
                if is_float {
                    BuiltinFunction::FloatMax
                } else {
                    BuiltinFunction::IntMax
                },
                one(owner, owner),
            )),
            (_, "clamp") => Some((
                if is_float {
                    BuiltinFunction::FloatClamp
                } else {
                    BuiltinFunction::IntClamp
                },
                Signature {
                    parameters: vec![("low".into(), owner), ("high".into(), owner)],
                    result: owner,
                },
            )),
            (false, "pow") => Some((BuiltinFunction::IntPow, one(int, int))),
            (true, "pow") => Some((BuiltinFunction::FloatPow, one(float, float))),
            (true, "floor") => Some((BuiltinFunction::FloatFloor, no_args(float))),
            (true, "ceil") => Some((BuiltinFunction::FloatCeil, no_args(float))),
            (true, "round") => Some((BuiltinFunction::FloatRound, no_args(float))),
            (true, "trunc") => Some((BuiltinFunction::FloatTrunc, no_args(float))),
            (true, "sqrt") => Some((BuiltinFunction::FloatSqrt, no_args(float))),
            (true, "to_int") => Some((BuiltinFunction::FloatToInt, no_args(int))),
            (true, "is_nan") => Some((BuiltinFunction::FloatIsNan, no_args(bool_ty))),
            (true, "is_finite") => Some((BuiltinFunction::FloatIsFinite, no_args(bool_ty))),
            _ => None,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "list method table is intentionally centralized"
    )]
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
            "pop" => Some((BuiltinFunction::ListPop, no_args(element))),
            "first" => Some((BuiltinFunction::ListFirst, no_args(element))),
            "last" => Some((BuiltinFunction::ListLast, no_args(element))),
            "index_of" => Some((
                BuiltinFunction::ListFindIndex,
                Signature {
                    parameters: vec![("value".into(), element)],
                    result: int,
                },
            )),
            "extend" => Some((
                BuiltinFunction::ListExtend,
                Signature {
                    parameters: vec![("other".into(), self.intern(Type::List(element)))],
                    result: void,
                },
            )),
            "reverse" => Some((BuiltinFunction::ListReverse, no_args(void))),
            "sort"
                if matches!(
                    &self.types[element.0],
                    Type::Int | Type::Float | Type::Numeric(_) | Type::Str | Type::Bool
                ) =>
            {
                Some((BuiltinFunction::ListSort, no_args(void)))
            }
            "truncate" => Some((
                BuiltinFunction::ListTruncate,
                Signature {
                    parameters: vec![("len".into(), int)],
                    result: void,
                },
            )),
            "swap" => Some((
                BuiltinFunction::ListSwap,
                Signature {
                    parameters: vec![("a".into(), int), ("b".into(), int)],
                    result: void,
                },
            )),
            "shrink_to_fit" => Some((BuiltinFunction::ListShrinkToFit, no_args(void))),
            "join" if matches!(&self.types[element.0], Type::Str) => Some((
                BuiltinFunction::ListJoin,
                Signature {
                    parameters: vec![("separator".into(), self.intern(Type::Str))],
                    result: self.intern(Type::Str),
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

    #[expect(
        clippy::too_many_lines,
        reason = "bytes method table is intentionally centralized"
    )]
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
            "byte_at" => Some((
                BuiltinFunction::BytesByteAt,
                Signature {
                    parameters: vec![("index".into(), int)],
                    result: int,
                },
            )),
            "push" => Some((
                BuiltinFunction::BytesPush,
                Signature {
                    parameters: vec![("value".into(), u8_ty)],
                    result: void,
                },
            )),
            "extend" => Some((
                BuiltinFunction::BytesExtend,
                Signature {
                    parameters: vec![("other".into(), bytes)],
                    result: void,
                },
            )),
            "truncate" => Some((
                BuiltinFunction::BytesTruncate,
                Signature {
                    parameters: vec![("len".into(), int)],
                    result: void,
                },
            )),
            "resize" => Some((
                BuiltinFunction::BytesResize,
                Signature {
                    parameters: vec![("len".into(), int), ("value".into(), u8_ty)],
                    result: void,
                },
            )),
            "find" => Some((
                BuiltinFunction::BytesFind,
                Signature {
                    parameters: vec![("needle".into(), bytes)],
                    result: int,
                },
            )),
            "starts_with" => Some((
                BuiltinFunction::BytesStartsWith,
                Signature {
                    parameters: vec![("prefix".into(), bytes)],
                    result: bool_ty,
                },
            )),
            "ends_with" => Some((
                BuiltinFunction::BytesEndsWith,
                Signature {
                    parameters: vec![("suffix".into(), bytes)],
                    result: bool_ty,
                },
            )),
            "compare" => Some((
                BuiltinFunction::BytesCompare,
                Signature {
                    parameters: vec![("other".into(), bytes)],
                    result: int,
                },
            )),
            "to_hex" => Some((BuiltinFunction::BytesToHex, no_args(string))),
            "clear" => Some((BuiltinFunction::BytesClear, no_args(void))),
            "to_list" => Some((
                BuiltinFunction::BytesToList,
                no_args(self.intern(Type::List(u8_ty))),
            )),
            "to_str" => Some((
                BuiltinFunction::BytesToStr,
                no_args(self.intern(Type::Result(string, utf8_error))),
            )),
            _ => self.bytes_endian_method(name),
        }
    }

    /// Resolves explicit-endian fixed-width byte accessors such as
    /// `read_u32_le` / `write_i64_be`.
    fn bytes_endian_method(&mut self, name: &str) -> Option<(BuiltinFunction, Signature)> {
        let (write, rest) = if let Some(rest) = name.strip_prefix("read_") {
            (false, rest)
        } else {
            (true, name.strip_prefix("write_")?)
        };
        let (numeric, endian) = rest.rsplit_once('_')?;
        let width = match numeric {
            "u16" | "i16" => 2_u8,
            "u32" | "i32" => 4,
            "u64" | "i64" => 8,
            _ => return None,
        };
        let signed = numeric.starts_with('i');
        let big_endian = match endian {
            "le" => false,
            "be" => true,
            _ => return None,
        };
        let int = self.intern(Type::Int);
        if write {
            Some((
                BuiltinFunction::BytesWriteInt { width, big_endian },
                Signature {
                    parameters: vec![("offset".into(), int), ("value".into(), int)],
                    result: self.intern(Type::Void),
                },
            ))
        } else {
            Some((
                BuiltinFunction::BytesReadInt {
                    width,
                    big_endian,
                    signed,
                },
                Signature {
                    parameters: vec![("offset".into(), int)],
                    result: int,
                },
            ))
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
                    result: self.intern(Type::Optional(value)),
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
                    result: self.intern(Type::Optional(value)),
                },
            )),
            "clear" => Some((BuiltinFunction::MapClear, no_args(void))),
            "keys" => Some((
                BuiltinFunction::MapKeys,
                no_args(self.intern(Type::List(key))),
            )),
            "values" => Some((
                BuiltinFunction::MapValues,
                no_args(self.intern(Type::List(value))),
            )),
            "get_or" => Some((
                BuiltinFunction::MapGetOr,
                Signature {
                    parameters: vec![("key".into(), key), ("default".into(), value)],
                    result: value,
                },
            )),
            _ => None,
        }
    }
}
