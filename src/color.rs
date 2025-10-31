
pub trait Colorize {
    fn red(&self) -> String;
    fn green(&self) -> String;
    fn blue(&self) -> String;
    fn yellow(&self) -> String;
}

impl Colorize for &str {
    fn red(&self) -> String {
        format!("\x1b[31m{}\x1b[0m", self)
    }
    fn green(&self) -> String {
        format!("\x1b[32m{}\x1b[0m", self)
    }
    fn blue(&self) -> String {
        format!("\x1b[34m{}\x1b[0m", self)
    }

    fn yellow(&self) -> String {
        format!("\x1b[33m{}\x1b[0m", self)
    }
}

impl Colorize for String {
    fn red(&self) -> String { self.as_str().red() }
    fn green(&self) -> String { self.as_str().green() }
    fn blue(&self) -> String { self.as_str().blue() }
    fn yellow(&self) -> String { self.as_str().yellow() }
}

#[test]
fn test_colorize() {
    let s = "proxy";
    println!("{}", s.red());
    println!("{}", s.green());
    println!("{}", s.blue());
    println!("{}", s.yellow());
}