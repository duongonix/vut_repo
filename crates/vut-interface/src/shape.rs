use std::collections::HashMap;
use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_hir::TypeId;
use vut_source::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterfaceMethod {
    pub parameters: Vec<TypeId>,
    pub result: TypeId,
    pub is_static: bool,
    pub span: Span,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InterfaceShape {
    pub methods: HashMap<String, InterfaceMethod>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Satisfaction {
    Satisfied,
    Missing { method: String },
    Mismatch { method: String },
    Private { method: String },
}

pub fn merge_requirement(
    diagnostics: &mut DiagnosticSink,
    shape: &mut InterfaceShape,
    name: String,
    method: InterfaceMethod,
    owner_span: Span,
) {
    if let Some(existing) = shape.methods.get(&name) {
        if existing.parameters != method.parameters || existing.result != method.result {
            diagnostics.push(
                Diagnostic::error(
                    codes::E4010,
                    "conflicting interface requirements",
                    owner_span,
                    format!("method `{name}` has incompatible inherited signatures"),
                )
                .with_related(existing.span, "first requirement")
                .with_related(method.span, "conflicting requirement"),
            );
        }
    } else {
        shape.methods.insert(name, method);
    }
}
