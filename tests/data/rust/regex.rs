use std::str::FromStr;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <N>", args[0]);
        std::process::exit(-1);
    }

    let size: usize = usize::from_str(&args[1]).unwrap();

    let input: String = std::iter::repeat('a').take(size).collect();
    for c in input.chars() {
        if c != 'a' {
            std::process::exit(1);
        }
    }
}
