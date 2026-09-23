//! Signature help resolved through semantic call targets.

use tower_lsp::lsp_types::{SignatureHelp, SignatureInformation};
use vut_source::Span;

use super::{
    Analysis,
    members::{active_parameter, call_context, open_call, parameter_infos, receiver_kind_at},
    presentation::symbol_signature,
};

pub(super) fn help(a: &Analysis, text: &str, at: usize) -> Option<SignatureHelp> {
    let open = open_call(text, at)?;
    let active = active_parameter(text, open, at);
    let rendered = semantic_call(a, open)
        .map(|span| symbol_signature(a, a.semantics.call_targets[&span]))
        .or_else(|| builtin_call(a, text, at))?;
    Some(SignatureHelp {
        signatures: vec![SignatureInformation {
            label: rendered.clone(),
            documentation: None,
            parameters: Some(parameter_infos(&rendered)),
            active_parameter: Some(active),
        }],
        active_signature: Some(0),
        active_parameter: Some(active),
    })
}

fn semantic_call(a: &Analysis, open: usize) -> Option<Span> {
    a.semantics
        .call_targets
        .keys()
        .filter(|span| {
            span.source() == a.current_source && span.start() <= open && open <= span.end()
        })
        .min_by_key(|span| span.end().saturating_sub(span.start()))
        .copied()
}

fn builtin_call(a: &Analysis, text: &str, at: usize) -> Option<String> {
    let call = call_context(text, at)?;
    let kind = receiver_kind_at(a, call.dot)?;
    vut_types::builtin_method_signature(kind, &call.member).map(ToOwned::to_owned)
}
