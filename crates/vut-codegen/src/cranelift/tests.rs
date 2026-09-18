//! Codegen backend tests.
use super::*;
use std::collections::HashMap;
use vut_mir::{
    AbiClass, BasicBlock, BlockId, Function as MirFunction, Instruction, LayoutTable,
    OwnershipKind, Program, Terminator, TypeInfo, ValueId, ValueRepr,
};
use vut_resolver::SymbolId;
fn constant() -> Program {
    Program {
        interface_vtables: Vec::new(),
        layouts: LayoutTable {
            pointer_size: 8,
            fields: HashMap::new(),
            arrays: HashMap::new(),
            lists: HashMap::new(),
            maps: HashMap::new(),
            results: HashMap::new(),
            enums: HashMap::new(),
            optionals: HashMap::new(),
            callables: HashMap::new(),
            aggregates: std::collections::HashSet::new(),
            types: vec![TypeInfo {
                size: 8,
                alignment: 8,
                is_copy: true,
                needs_drop: false,
                contains_managed: false,
                contains_linear: false,
                abi: AbiClass::Scalar,
                repr: ValueRepr::Integer,
                ownership: OwnershipKind::None,
            }],
        },
        external_functions: Vec::new(),
        diagnostics: vut_mir::DiagnosticSink::new(),
        frames: HashMap::new(),
        awaits: HashMap::new(),
        functions: vec![MirFunction {
            symbol: SymbolId(0),
            receiver: None,
            parameter_count: 0,
            locals: vec![],
            entry: BlockId(0),
            return_type: Some(vut_hir::TypeId(0)),
            is_async: false,
            frame_param: None,
            is_poll: false,
            out_param: None,
            blocks: vec![BasicBlock {
                instructions: vec![Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 42,
                }],
                terminator: Terminator::Return(Some(ValueId(0))),
            }],
        }],
    }
}
#[test]
fn emits_a_real_host_object() {
    let bytes = CraneliftBackend::new(Target::host())
        .compile_module(&constant())
        .unwrap();
    assert!(bytes.len() > 64);
    assert!(
        bytes.starts_with(b"\x7fELF")
            || bytes.starts_with(b"MZ")
            || bytes.starts_with(b"\x64\x86")
            || bytes.starts_with(b"\x4c\x01")
            || &bytes[..4] == b"\xcf\xfa\xed\xfe"
            || &bytes[..4] == b"\xfe\xed\xfa\xcf"
    );
}

/// Builds a program whose only work is calling an external symbol, which forces
/// the backend to emit a relocation for that symbol reference.
fn calls_external() -> Program {
    let mut program = constant();
    program.functions[0].blocks[0] = BasicBlock {
        instructions: vec![Instruction::Call {
            value: Some(ValueId(0)),
            result_type: None,
            target: SymbolId(7),
            arguments: vec![],
        }],
        terminator: Terminator::Return(Some(ValueId(0))),
    };
    program.external_functions.push(vut_mir::ExternalFunction {
        symbol: SymbolId(7),
        link_name: "native_add".into(),
        parameters: vec![],
        return_type: Some(vut_hir::TypeId(0)),
    });
    program
}

/// Executable sections must not contain absolute relocations (text
/// relocations). macOS arm64 rejects them outright; other platforms require
/// position-independent objects to link cleanly, so this is a cross-platform
/// regression guard for the `is_pic` codegen setting.
#[test]
fn emits_position_independent_code() {
    use object::{Object as _, ObjectSection as _, RelocationKind, SectionKind};

    let bytes = CraneliftBackend::new(Target::host())
        .compile_module(&calls_external())
        .unwrap();
    let file = object::File::parse(&*bytes).expect("parse emitted object");

    let mut saw_text = false;
    for section in file.sections() {
        if section.kind() != SectionKind::Text {
            continue;
        }
        saw_text = true;
        for (offset, relocation) in section.relocations() {
            assert_ne!(
                relocation.kind(),
                RelocationKind::Absolute,
                "absolute relocation at {offset:#x} in executable section `{}`",
                section.name().unwrap_or("<unnamed>")
            );
        }
    }
    assert!(saw_text, "expected at least one executable section");
}
#[test]
fn emits_static_utf8_string_data() {
    let mut program = constant();
    program.functions[0].blocks[0].instructions = vec![Instruction::ConstString {
        value: ValueId(0),
        literal: "Việt".into(),
    }];
    let object = CraneliftBackend::new(Target::host())
        .compile_module(&program)
        .unwrap();
    assert!(object.windows(6).any(|bytes| bytes == "Việt".as_bytes()));
}
#[test]
fn rejects_runtime_abi_mismatch() {
    assert!(matches!(
        verify_runtime_abi(99),
        Err(CodegenError::RuntimeAbi { .. })
    ));
}
#[test]
fn lowers_branches_comparisons_and_direct_calls() {
    let mut program = constant();
    program.functions.push(MirFunction {
        symbol: SymbolId(1),
        receiver: None,
        parameter_count: 0,
        locals: vec![],
        entry: BlockId(0),
        return_type: Some(vut_hir::TypeId(0)),
        is_async: false,
        frame_param: None,
        is_poll: false,
        out_param: None,
        blocks: vec![
            BasicBlock {
                instructions: vec![Instruction::ConstInt {
                    value: ValueId(0),
                    literal: 1,
                }],
                terminator: Terminator::Branch {
                    condition: ValueId(0),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                },
            },
            BasicBlock {
                instructions: vec![Instruction::Call {
                    value: Some(ValueId(1)),
                    result_type: None,
                    target: SymbolId(0),
                    arguments: vec![],
                }],
                terminator: Terminator::Return(Some(ValueId(1))),
            },
            BasicBlock {
                instructions: vec![Instruction::ConstInt {
                    value: ValueId(2),
                    literal: 0,
                }],
                terminator: Terminator::Return(Some(ValueId(2))),
            },
        ],
    });
    let object = CraneliftBackend::new(Target::host())
        .compile_module(&program)
        .unwrap();
    assert!(object.len() > 64);
}

#[test]
fn declares_extern_functions_by_link_name() {
    let mut program = constant();
    program.functions.clear();
    program.external_functions.push(vut_mir::ExternalFunction {
        symbol: SymbolId(7),
        link_name: "native_add".into(),
        parameters: vec![vut_hir::TypeId(0), vut_hir::TypeId(0)],
        return_type: Some(vut_hir::TypeId(0)),
    });
    let object = CraneliftBackend::new(Target::host())
        .compile_module(&program)
        .unwrap();
    assert!(
        object
            .windows("native_add".len())
            .any(|bytes| bytes == b"native_add")
    );
}

#[test]
fn lowers_aggregate_construction_and_field_access_from_layout_table() {
    use vut_hir::TypeId;
    use vut_mir::FieldLayout;
    let mut program = constant();
    program.layouts.types.push(TypeInfo {
        size: 8,
        alignment: 8,
        is_copy: true,
        needs_drop: false,
        contains_managed: false,
        contains_linear: false,
        abi: AbiClass::Scalar,
        repr: ValueRepr::Integer,
        ownership: OwnershipKind::None,
    });
    program.layouts.fields.insert(
        TypeId(1),
        vec![FieldLayout {
            name: "value".into(),
            ty: TypeId(0),
            offset: 0,
        }],
    );
    program.functions[0].blocks[0] = BasicBlock {
        instructions: vec![
            Instruction::ConstInt {
                value: ValueId(0),
                literal: 42,
            },
            Instruction::Allocate {
                value: ValueId(1),
                ty: TypeId(1),
            },
            Instruction::Construct {
                value: ValueId(1),
                ty: TypeId(1),
                fields: vec![("value".into(), ValueId(0))],
            },
            Instruction::Field {
                value: ValueId(2),
                base: ValueId(1),
                name: "value".into(),
            },
        ],
        terminator: Terminator::Return(Some(ValueId(2))),
    };
    let object = CraneliftBackend::new(Target::host())
        .compile_module(&program)
        .unwrap();
    assert!(object.len() > 64);
}
