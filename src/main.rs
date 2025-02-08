fn main() {
    if let Err(e) = scaficionado::run() {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
