//! Backend-independent, resolved high-level representation.
use vut_ast::{Attribute, Block, DataField, TypeExpr};
use vut_resolver::{ModuleId, Resolution, ResolvedReference, SymbolId};
use vut_source::{SourceId, Span};

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(pub usize);
    };
}
id!(HirId);
id!(FunctionId);
id!(DataId);
id!(InterfaceId);
id!(EnumId);
id!(TypeId);

#[derive(Clone, Debug)]
pub struct HirProgram {
    pub modules: Vec<HirModule>,
    pub resolved_references: Vec<ResolvedReference>,
    pub lambda_symbols: std::collections::HashMap<Span, SymbolId>,
}
#[derive(Clone, Debug)]
pub struct HirModule {
    pub id: ModuleId,
    pub source: SourceId,
    pub imports: Vec<ModuleId>,
    pub declarations: Vec<HirDeclaration>,
}
#[derive(Clone, Debug)]
#[expect(
    clippy::large_enum_variant,
    reason = "declarations are owned once per module; boxing would add indirection"
)]
pub enum HirDeclaration {
    Function(HirFunction),
    Data(HirData),
    Interface(HirInterface),
    Enum(HirEnum),
    TypeAlias(HirTypeAlias),
}
#[derive(Clone, Debug)]
pub struct HirFunction {
    pub id: FunctionId,
    pub symbol: SymbolId,
    pub receiver: Option<SymbolId>,
    pub implicit_self: Option<HirId>,
    pub is_async: bool,
    /// `true` for a lifted lambda body. Closure bodies receive a hidden leading
    /// environment parameter and load their captures from it.
    pub is_closure: bool,
    /// Enclosing bindings captured by a closure body, in first-use order.
    pub captures: Vec<HirCapture>,
    pub type_parameters: Vec<HirTypeParameter>,
    pub parameters: Vec<HirParameter>,
    pub return_type: Option<TypeExpr>,
    pub body: Option<Block>,
    pub external_abi: Option<String>,
    /// Declared name used as the default native symbol for extern functions.
    pub external_name: Option<String>,
    pub attributes: Vec<Attribute>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct HirCapture {
    pub name: String,
    pub symbol: SymbolId,
}
#[derive(Clone, Debug)]
pub struct HirParameter {
    pub id: HirId,
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}
/// A generic type parameter carried from source (`T` or `T: Bound`).
#[derive(Clone, Debug)]
pub struct HirTypeParameter {
    pub name: String,
    pub bounds: Vec<TypeExpr>,
}
#[derive(Clone, Debug)]
pub struct HirData {
    pub id: DataId,
    pub symbol: SymbolId,
    pub attributes: Vec<Attribute>,
    pub type_parameters: Vec<HirTypeParameter>,
    pub fields: Vec<DataField>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct HirInterface {
    pub id: InterfaceId,
    pub symbol: SymbolId,
    pub attributes: Vec<Attribute>,
    pub type_parameters: Vec<HirTypeParameter>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct HirEnum {
    pub id: EnumId,
    pub symbol: SymbolId,
    pub attributes: Vec<Attribute>,
    pub type_parameters: Vec<HirTypeParameter>,
    pub variants: Vec<String>,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub struct HirTypeAlias {
    pub symbol: SymbolId,
    pub attributes: Vec<Attribute>,
    pub target: TypeExpr,
    pub span: Span,
}

/// Lowers resolver output without re-resolving names or imports.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "lowering keeps declaration variants together"
)]
pub fn lower(resolution: &Resolution) -> HirProgram {
    let mut function_index = 0;
    let mut data_index = 0;
    let mut interface_index = 0;
    let mut enum_index = 0;
    let mut hir_index = 0;
    let mut modules = Vec::with_capacity(resolution.modules.len());
    for module in &resolution.modules {
        let mut declarations = Vec::new();
        for item in &module.file.items {
            match item {
                vut_ast::Item::Function(value) => {
                    let Some(symbol) = module.symbols.get(&value.name.text).copied() else {
                        continue;
                    };
                    declarations.push(HirDeclaration::Function(HirFunction {
                        id: FunctionId(function_index),
                        symbol,
                        receiver: None,
                        implicit_self: None,
                        is_async: value.is_async,
                        is_closure: false,
                        captures: Vec::new(),
                        type_parameters: lower_type_parameters(&value.type_parameters),
                        parameters: lower_parameters(&value.parameters, &mut hir_index),
                        return_type: value.return_type.clone(),
                        body: Some(value.body.clone()),
                        external_abi: None,
                        external_name: None,
                        attributes: value.attributes.clone(),
                        span: value.span,
                    }));
                    function_index += 1;
                }
                vut_ast::Item::ExternFunction(value) => {
                    let Some(symbol) = module.symbols.get(&value.name.text).copied() else {
                        continue;
                    };
                    declarations.push(HirDeclaration::Function(HirFunction {
                        id: FunctionId(function_index),
                        symbol,
                        receiver: None,
                        implicit_self: None,
                        is_async: value.is_async,
                        is_closure: false,
                        captures: Vec::new(),
                        type_parameters: lower_type_parameters(&value.type_parameters),
                        parameters: lower_parameters(&value.parameters, &mut hir_index),
                        return_type: value.return_type.clone(),
                        body: None,
                        external_abi: Some(value.abi.clone()),
                        external_name: Some(value.name.text.clone()),
                        attributes: value.attributes.clone(),
                        span: value.span,
                    }));
                    function_index += 1;
                }
                vut_ast::Item::Method(value) => {
                    let method = module.methods.iter().find(|(key, symbol)| {
                        key.name == value.name.text
                            && resolution.symbols[symbol.0].span == value.name.span
                    });
                    let Some((key, symbol)) = method else {
                        continue;
                    };
                    let self_id = HirId(hir_index);
                    hir_index += 1;
                    declarations.push(HirDeclaration::Function(HirFunction {
                        id: FunctionId(function_index),
                        symbol: *symbol,
                        receiver: if value.is_static {
                            None
                        } else {
                            Some(key.receiver)
                        },
                        implicit_self: if value.is_static { None } else { Some(self_id) },
                        is_async: value.is_async,
                        is_closure: false,
                        captures: Vec::new(),
                        type_parameters: lower_type_parameters(&value.type_parameters),
                        parameters: lower_parameters(&value.parameters, &mut hir_index),
                        return_type: value.return_type.clone(),
                        body: Some(value.body.clone()),
                        external_abi: None,
                        external_name: None,
                        attributes: value.attributes.clone(),
                        span: value.span,
                    }));
                    function_index += 1;
                }
                vut_ast::Item::Data(value) => {
                    if let Some(symbol) = module.symbols.get(&value.name.text).copied() {
                        declarations.push(HirDeclaration::Data(HirData {
                            id: DataId(data_index),
                            symbol,
                            attributes: value.attributes.clone(),
                            type_parameters: lower_type_parameters(&value.type_parameters),
                            fields: value.fields.clone(),
                            span: value.span,
                        }));
                        data_index += 1;
                    }
                }
                vut_ast::Item::Interface(value) => {
                    if let Some(symbol) = module.symbols.get(&value.name.text).copied() {
                        declarations.push(HirDeclaration::Interface(HirInterface {
                            id: InterfaceId(interface_index),
                            symbol,
                            attributes: value.attributes.clone(),
                            type_parameters: lower_type_parameters(&value.type_parameters),
                            span: value.span,
                        }));
                        interface_index += 1;
                    }
                }
                vut_ast::Item::Enum(value) => {
                    if let Some(symbol) = module.symbols.get(&value.name.text).copied() {
                        declarations.push(HirDeclaration::Enum(HirEnum {
                            id: EnumId(enum_index),
                            symbol,
                            attributes: value.attributes.clone(),
                            type_parameters: lower_type_parameters(&value.type_parameters),
                            variants: value
                                .variants
                                .iter()
                                .map(|variant| variant.name.text.clone())
                                .collect(),
                            span: value.span,
                        }));
                        enum_index += 1;
                    }
                }
                vut_ast::Item::TypeAlias(value) => {
                    if let Some(symbol) = module.symbols.get(&value.name.text).copied() {
                        declarations.push(HirDeclaration::TypeAlias(HirTypeAlias {
                            symbol,
                            attributes: value.attributes.clone(),
                            target: value.ty.clone(),
                            span: value.span,
                        }));
                    }
                }
                vut_ast::Item::Import(_) | vut_ast::Item::Statement(_) => {}
            }
        }
        for lambda in resolution.lambdas.iter().filter(|l| l.module == module.id) {
            let implicit_self = if lambda.receiver.is_some() {
                let id = HirId(hir_index);
                hir_index += 1;
                Some(id)
            } else {
                None
            };
            declarations.push(HirDeclaration::Function(HirFunction {
                id: FunctionId(function_index),
                symbol: lambda.symbol,
                receiver: lambda.receiver,
                implicit_self,
                is_async: lambda.is_async,
                is_closure: true,
                captures: lambda
                    .captures
                    .iter()
                    .map(|capture| HirCapture {
                        name: capture.name.clone(),
                        symbol: capture.symbol,
                    })
                    .collect(),
                type_parameters: Vec::new(),
                parameters: lower_lambda_parameters(&lambda.parameters, &mut hir_index),
                return_type: None,
                body: Some(lambda_body_block(&lambda.body)),
                external_abi: None,
                external_name: None,
                attributes: Vec::new(),
                span: lambda.span,
            }));
            function_index += 1;
        }
        modules.push(HirModule {
            id: module.id,
            source: module.source,
            imports: resolution.graph[module.id.0].clone(),
            declarations,
        });
    }
    HirProgram {
        modules,
        resolved_references: resolution.references.clone(),
        lambda_symbols: resolution.lambda_symbols.clone(),
    }
}

fn lower_type_parameters(parameters: &[vut_ast::TypeParameter]) -> Vec<HirTypeParameter> {
    parameters
        .iter()
        .map(|parameter| HirTypeParameter {
            name: parameter.name.text.clone(),
            bounds: parameter.bounds.clone(),
        })
        .collect()
}

fn lower_lambda_parameters(
    parameters: &[vut_ast::LambdaParameter],
    next: &mut usize,
) -> Vec<HirParameter> {
    parameters
        .iter()
        .map(|parameter| {
            let id = HirId(*next);
            *next += 1;
            HirParameter {
                id,
                name: parameter.name.text.clone(),
                ty: parameter
                    .ty
                    .clone()
                    .unwrap_or(TypeExpr::Error(parameter.span)),
                span: parameter.span,
            }
        })
        .collect()
}

fn lambda_body_block(body: &vut_ast::LambdaBody) -> Block {
    match body {
        vut_ast::LambdaBody::Block(block) => block.clone(),
        vut_ast::LambdaBody::Expression(value) => {
            let span = value.span();
            Block {
                statements: vec![vut_ast::Stmt::Expression((**value).clone())],
                span,
            }
        }
    }
}

fn lower_parameters(parameters: &[vut_ast::Parameter], next: &mut usize) -> Vec<HirParameter> {
    parameters
        .iter()
        .map(|parameter| {
            let id = HirId(*next);
            *next += 1;
            HirParameter {
                id,
                name: parameter.name.text.clone(),
                ty: parameter.ty.clone(),
                span: parameter.span,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vut_lexer::Lexer;
    use vut_parser::Parser;
    use vut_resolver::{ModuleInput, ModulePath, Resolver};

    #[test]
    fn lifts_lambda_expressions_into_anonymous_functions() {
        let source = SourceId::from_index(0);
        let text = "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(3, x => x * 2))\")\n";
        let (tokens, lexical) = Lexer::new(source, text).lex();
        assert!(!lexical.has_errors());
        let (file, syntax) = Parser::new(source, text, tokens).parse();
        assert!(!syntax.has_errors(), "{:?}", syntax.as_slice());
        let resolution = Resolver::new(vec![ModuleInput {
            logical_path: ModulePath(vec!["main".into()]),
            filesystem_path: None,
            file,
        }])
        .resolve();
        assert!(!resolution.diagnostics.has_errors());
        let hir = lower(&resolution);
        assert_eq!(hir.lambda_symbols.len(), 1);
        let symbol = *hir.lambda_symbols.values().next().expect("lambda symbol");
        assert!(
            hir.modules[0]
                .declarations
                .iter()
                .any(|declaration| matches!(
                    declaration,
                    HirDeclaration::Function(function) if function.symbol == symbol
                ))
        );
    }

    #[test]
    fn lowers_methods_with_structured_receiver_and_implicit_self() {
        let source = SourceId::from_index(0);
        let text = "data Counter:\n  value: int\nfn Counter.get() -> int:\n  self.value\n";
        let (tokens, lexical) = Lexer::new(source, text).lex();
        assert!(!lexical.has_errors());
        let (file, syntax) = Parser::new(source, text, tokens).parse();
        assert!(!syntax.has_errors());
        let resolution = Resolver::new(vec![ModuleInput {
            logical_path: ModulePath(vec!["main".into()]),
            filesystem_path: None,
            file,
        }])
        .resolve();
        assert!(!resolution.diagnostics.has_errors());
        let hir = lower(&resolution);
        let method = hir.modules[0]
            .declarations
            .iter()
            .find_map(|decl| match decl {
                HirDeclaration::Function(function) if function.receiver.is_some() => Some(function),
                _ => None,
            })
            .expect("method HIR");
        assert!(method.implicit_self.is_some());
        assert_eq!(
            method.receiver,
            resolution.symbols[method.symbol.0].receiver
        );
        assert_eq!(method.span.source(), source);
    }
}
