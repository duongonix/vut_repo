use std::{env, fs, io::{self, Read, Write}, thread, time::Duration};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args[1].as_str() {
        "marker" => {
            thread::sleep(Duration::from_millis(300));
            fs::write(&args[2], b"alive").unwrap();
        }
        "output" => {
            io::stdout().write_all(b"hello\0world").unwrap();
            io::stderr().write_all(b"error").unwrap();
        }
        "echo" => {
            let mut bytes = Vec::new();
            io::stdin().read_to_end(&mut bytes).unwrap();
            io::stdout().write_all(&bytes).unwrap();
            io::stderr().write_all(b"closed").unwrap();
        }
        "env" => println!("{}", env::var("VUT_M13_CHILD_TEST").unwrap_or_else(|_| "absent".into())),
        "config" => {
            println!("{}", args[2]);
            println!("{}", env::current_dir().unwrap().join("marker").is_file());
        }
        "sleep" => thread::sleep(Duration::from_secs(2)),
        "success" => {},
        _ => std::process::exit(7),
    }
}
