//! Module discovery, import resolution, scopes, and symbol binding.
mod loader;
mod model;
mod resolve;
mod scope;
pub use loader::{DiscoverError, LoadedRoot, load_source_file, load_source_root};
pub use model::{
    ImplicitReceiverMethod, LambdaInfo, MethodKey, Module, ModuleId, ModuleInput, ModulePath,
    Resolution, ResolvedReference, Symbol, SymbolId, SymbolKind,
};
pub use resolve::Resolver;

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };
    use vut_lexer::Lexer;
    use vut_parser::Parser;
    use vut_source::{SourceId, SourceManager};

    fn module(index: usize, path: &[&str], source: &str) -> ModuleInput {
        let source_id = SourceId::from_index(index);
        let (tokens, lexical) = Lexer::new(source_id, source).lex();
        assert!(!lexical.has_errors(), "{:?}", lexical.as_slice());
        let (file, syntax) = Parser::new(source_id, source, tokens).parse();
        assert!(!syntax.has_errors(), "{:?}", syntax.as_slice());
        ModuleInput {
            logical_path: ModulePath(path.iter().map(|item| (*item).to_owned()).collect()),
            filesystem_path: None,
            file,
        }
    }
    fn codes(resolution: &Resolution) -> Vec<&str> {
        resolution
            .diagnostics
            .as_slice()
            .iter()
            .filter_map(|item| item.code.map(vut_diagnostics::DiagnosticCode::as_str))
            .collect()
    }

    #[test]
    fn lifts_lambdas_to_anonymous_symbols_and_rejects_captures() {
        let resolution = Resolver::new(vec![module(
            0,
            &["main"],
            "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(3, x => x * 2))\")\n",
        )])
        .resolve();
        assert!(
            !resolution.diagnostics.has_errors(),
            "{:?}",
            resolution.diagnostics.as_slice()
        );
        assert_eq!(resolution.lambdas.len(), 1);
        let lambda = &resolution.lambdas[0];
        assert_eq!(
            resolution.symbols[lambda.symbol.0].kind,
            SymbolKind::AnonymousFunction
        );
        assert_eq!(
            resolution.lambda_symbols.get(&lambda.span).copied(),
            Some(lambda.symbol)
        );

        let capture = Resolver::new(vec![module(
            0,
            &["main"],
            "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  base = 10\n  out(\"$(apply(3, x => x + base))\")\n",
        )])
        .resolve();
        assert!(codes(&capture).contains(&"E1013"), "{:?}", codes(&capture));
    }

    #[test]
    fn resolves_absolute_relative_alias_and_selected_imports() {
        let resolution = Resolver::new(vec![
            module(0, &["math"], "fn add(a: int, b: int) -> int:\n  a + b\nfn _fast() -> int:\n  1\n"),
            module(1, &["rootutil"], "fn root() -> int:\n  1\n"),
            module(2, &["a", "utils"], "fn help() -> int:\n  1\n"),
            module(3, &["a", "b", "helper"], "fn local() -> int:\n  1\n"),
            module(4, &["a", "b", "main"], "import math at add\nimport ..utils as u\nimport .helper as h\nimport ...rootutil as r\nfn run() -> int:\n  add(1, 2) + u.help() + h.local() + r.root()\n"),
        ]).resolve();
        assert!(
            !resolution.diagnostics.has_errors(),
            "{:?}",
            resolution.diagnostics.as_slice()
        );
        let main = resolution
            .modules
            .iter()
            .find(|module| module.logical_path.display() == "a.b.main")
            .unwrap()
            .id;
        assert_eq!(resolution.graph[main.0].len(), 4);
        let main_position = resolution
            .compile_order
            .iter()
            .position(|module| *module == main)
            .unwrap();
        assert!(resolution.graph[main.0].iter().all(|dependency| {
            resolution
                .compile_order
                .iter()
                .position(|module| module == dependency)
                .unwrap()
                < main_position
        }));
        assert!(
            resolution
                .references
                .iter()
                .any(|reference| resolution.symbols[reference.symbol.0].name == "add")
        );
        assert!(
            resolution
                .references
                .iter()
                .any(|reference| resolution.symbols[reference.symbol.0].name == "u")
        );
    }

    #[test]
    fn reports_import_failures() {
        let resolution = Resolver::new(vec![
            module(
                0,
                &["math"],
                "fn add() -> int:\n  1\nfn _fast() -> int:\n  1\n",
            ),
            module(
                1,
                &["app", "main"],
                "fn add() -> int:\n  1\nimport math at add, missing, _fast\nimport absent\nimport ...outside\n",
            ),
        ])
        .resolve();
        let actual = codes(&resolution);
        for expected in ["E3001", "E3002", "E3003", "E3004", "E3006"] {
            assert!(actual.contains(&expected), "missing {expected}: {actual:?}");
        }
    }

    #[test]
    fn detects_full_import_cycle() {
        let resolution = Resolver::new(vec![
            module(0, &["a"], "import b\n"),
            module(1, &["b"], "import c\n"),
            module(2, &["c"], "import a\n"),
        ])
        .resolve();
        let diagnostic = resolution
            .diagnostics
            .as_slice()
            .iter()
            .find(|item| item.code.map(vut_diagnostics::DiagnosticCode::as_str) == Some("E3010"))
            .unwrap();
        assert!(
            diagnostic
                .primary
                .as_ref()
                .unwrap()
                .message
                .contains("a -> b -> c -> a")
        );
    }

    #[test]
    fn resolves_nested_scopes_and_suggests_typos() {
        let resolution = Resolver::new(vec![module(0, &["main"], "fn greet(name: str):\n  prefix = \"Hi\"\n  if true:\n    out(\"$prefix $name\")\n  pritn(name)\n")]).resolve();
        let unknown = resolution
            .diagnostics
            .as_slice()
            .iter()
            .find(|item| item.code.map(vut_diagnostics::DiagnosticCode::as_str) == Some("E2001"))
            .unwrap();
        assert_eq!(unknown.help.as_deref(), Some("did you mean `print`?"));
        assert_eq!(resolution.diagnostics.error_count(), 1);
    }

    #[test]
    fn discovers_project_and_dependency_roots_deterministically() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("vut-resolver-{nonce}"));
        let project = directory.join("src");
        let package = directory.join("package-src");
        fs::create_dir_all(project.join("nested")).unwrap();
        fs::create_dir_all(&package).unwrap();
        fs::write(project.join("main.vut"), "import math\n").unwrap();
        fs::write(project.join("nested").join("util.vut"), "VALUE = 1\n").unwrap();
        fs::write(package.join("lib.vut"), "fn add() -> int:\n  1\n").unwrap();
        let mut sources = SourceManager::new();
        let mut inputs = load_source_root(&project, &[], &mut sources)
            .unwrap()
            .modules;
        inputs.extend(
            load_source_root(&package, &["math".into()], &mut sources)
                .unwrap()
                .modules,
        );
        let resolution = Resolver::new(inputs).resolve();
        assert!(
            !resolution.diagnostics.has_errors(),
            "{:?}",
            resolution.diagnostics.as_slice()
        );
        assert_eq!(
            resolution
                .modules
                .iter()
                .map(|item| item.logical_path.display())
                .collect::<Vec<_>>(),
            vec!["main", "math", "nested.util"]
        );
        fs::remove_file(project.join("main.vut")).unwrap();
        fs::remove_file(project.join("nested").join("util.vut")).unwrap();
        fs::remove_file(package.join("lib.vut")).unwrap();
        fs::remove_dir(project.join("nested")).unwrap();
        fs::remove_dir(project).unwrap();
        fs::remove_dir(package).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn associates_methods_with_resolved_receiver_namespaces() {
        let resolution = Resolver::new(vec![module(
            0,
            &["main"],
            "data User:\n  name: str\ndata File:\n  path: str\nfn User.open():\n  self.name\nfn File.open():\n  self.path\n",
        )])
        .resolve();
        assert!(
            !resolution.diagnostics.has_errors(),
            "{:?}",
            resolution.diagnostics.as_slice()
        );
        let module = &resolution.modules[0];
        assert_eq!(module.methods.len(), 2);
        let receivers: Vec<_> = module
            .methods
            .keys()
            .map(|key| resolution.symbols[key.receiver.0].name.as_str())
            .collect();
        assert!(receivers.contains(&"User"));
        assert!(receivers.contains(&"File"));
        assert!(module.methods.keys().all(|key| key.name == "open"));
    }

    #[test]
    fn rejects_unknown_and_duplicate_method_receivers() {
        let resolution = Resolver::new(vec![module(
            0,
            &["main"],
            "data User:\n  name: str\nfn User.save():\n  self.name\nfn User.save():\n  self.name\nfn Missing.save():\n  1\n",
        )])
        .resolve();
        let actual = codes(&resolution);
        assert!(actual.contains(&"E2008"), "{actual:?}");
        assert!(actual.contains(&"E2001"), "{actual:?}");
    }

    #[test]
    fn rejects_self_outside_instance_methods() {
        let resolution =
            Resolver::new(vec![module(0, &["main"], "fn invalid():\n  self\n")]).resolve();
        assert!(codes(&resolution).contains(&"E2007"));
    }

    #[test]
    fn rejects_extension_methods_for_imported_types() {
        let resolution = Resolver::new(vec![
            module(0, &["models"], "data User:\n  name: str\n"),
            module(
                1,
                &["main"],
                "import models at User\nfn User.rename(name: str):\n  self.name = name\n",
            ),
        ])
        .resolve();
        let diagnostic = resolution
            .diagnostics
            .as_slice()
            .iter()
            .find(|item| item.title == "method receiver is not local")
            .expect("extension method must be rejected");
        assert_eq!(diagnostic.code.as_deref(), Some("E2001"));
    }
}
