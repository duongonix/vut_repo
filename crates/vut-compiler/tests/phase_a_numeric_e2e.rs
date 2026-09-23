use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn run(source: &str) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-phase-a-numeric-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .expect("emit executable");
    let output = Command::new(&executable).output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

#[test]
fn int_math_methods() {
    let source = "fn main():\n  n: int = 0 - 7\n  out(\"abs=$(n.abs())\")\n  base = 3\n  out(\"pow=$(base.pow(4))\")\n  out(\"min=$(base.min(5)) max=$(base.max(5)) clamp=$(base.clamp(1, 2))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "abs=7\npow=81\nmin=3 max=5 clamp=2\n");
}

#[test]
fn float_math_methods() {
    let source = "fn main():\n  f = 2.5\n  out(\"floor=$(f.floor()) ceil=$(f.ceil()) round=$(f.round())\")\n  nine: float = 9.0\n  out(\"sqrt=$(nine.sqrt())\")\n  neg: float = 0.0 - 2.5\n  out(\"fabs=$(neg.abs())\")\n  out(\"finite=$(f.is_finite()) nan=$(f.is_nan()) to_int=$(f.to_int())\")\n  root = neg.sqrt()\n  out(\"nan2=$(root.is_nan()) finite2=$(root.is_finite())\")\n  out(\"pow=$(base_pow())\")\n\nfn base_pow() -> float:\n  base: float = 2.0\n  base.pow(10.0)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "floor=2 ceil=3 round=3\nsqrt=3\nfabs=2.5\nfinite=1 nan=0 to_int=2\nnan2=1 finite2=0\npow=1024\n"
    );
}

#[test]
fn result_helpers() {
    let source = "fn main():\n  okv: result[int, str] = ok(7)\n  out(\"is_ok=$(okv.is_ok()) is_err=$(okv.is_err()) unwrap=$(okv.unwrap_or(0))\")\n  errv: result[int, str] = err(\"x\")\n  out(\"is_ok2=$(errv.is_ok()) unwrap2=$(errv.unwrap_or(99))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "is_ok=1 is_err=0 unwrap=7\nis_ok2=0 unwrap2=99\n");
}

#[test]
fn result_unwrap_or_with_managed_default_does_not_leak() {
    let source = "fn main():\n  fallback = \"none\"\n  good: result[str, int] = ok(\"hi\")\n  good_out = good.unwrap_or(fallback)\n  out(\"good=$good_out\")\n  bad: result[str, int] = err(1)\n  bad_out = bad.unwrap_or(fallback)\n  out(\"bad=$bad_out\")\n  out(\"bool=$(false.to_str())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "good=hi\nbad=none\nbool=false\n");
}

#[test]
fn numeric_loop_does_not_leak() {
    let source = "fn main():\n  total = 0\n  index = 1\n  for index < 20000:\n    total = total + (0 - index).abs()\n    index = index + 1\n  out(\"total=$total\")\n  acc: float = 0.0\n  step = 1\n  for step < 1000:\n    acc = acc + (step.to_float()).sqrt()\n    step = step + 1\n  out(\"sqrt_sum=$(acc.to_int())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(stdout, "total=199990000\nsqrt_sum=21065\n");
}

#[test]
fn fixed_width_literals_conversions_and_unsigned_operations() {
    let source = "fn main():\n  low: i8 = -128\n  high: u64 = 18446744073709551615\n  narrow: f32 = 1.5\n  wide = narrow.to_float()\n  again = wide.to_f32()\n  out(\"$(low.to_i64()) $high $(high / 3) $(high > 1) $(again.to_str())\")\n  x: u8 = 10\n  out(\"$(x.to_i16().to_i32().to_i64().to_int().to_u8().to_u16().to_u32().to_u64().to_usize().to_f32().to_f64().to_float())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        stdout,
        "-128 18446744073709551615 6148914691236517205 1 1.5\n10\n"
    );
}

#[test]
fn negative_signed_to_unsigned_conversion_traps() {
    let (code, _) = run("fn main():\n  value: i64 = -1\n  out(value.to_u64())\n");
    assert_ne!(code, Some(0));
}
