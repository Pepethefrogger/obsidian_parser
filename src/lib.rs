use std::fs;

use regex::Regex;

#[derive(Debug,PartialEq)]
enum Preprocessor {
    Text(String),
    Header(String),
    CodeBlock(String),
    Table(Vec<Vec<String>>),
    Callout(String),
    BulletList(Vec<String>),
    NumberedList(Vec<String>),
    TaskList(Vec<String>),
}

#[derive(Debug,PartialEq)]
enum Section {
    Text(String),
    Header(u32,String),
    CodeBlock(String,String),
    Table(Vec<Vec<String>>),
    Callout(String),
    BulletList(Vec<(u32,String)>),
    NumberedList(Vec<(u32,String)>),
    TaskList(Vec<(u32,bool,String)>)
}

struct Parser {
    node: PreprocessedNode,
    current: Vec<String>,
    current_table: Vec<Vec<String>>,
    codeblock: bool,
    table: bool,
    callout: bool,
    bulletlist: bool,
    numberedlist: bool,
    tasklist: bool
}

impl Parser {
    fn new() -> Self {
        Parser {node:PreprocessedNode { path: String::new(), content:vec![] },current:vec![],current_table:vec![],codeblock:false,table:false,callout:false,bulletlist:false,numberedlist:false,tasklist:false}
    }

    fn write(&mut self,section:Preprocessor) {
        if let Preprocessor::Text(s) = &section {
            if s.is_empty() {return};
        }
        match &section {
            Preprocessor::Table(_) => self.current_table.clear(),
            _ => self.current.clear()
        }
        self.node.content.push(section);
    }

    fn current_joined(&self) -> String {
        String::from(self.current.join("\n").trim())
    }

    fn parse(mut self,path: String) -> PreprocessedNode {
        self.node.path = path.clone();
        let tasklist_reg = Regex::new(r"^- \[[ x]\] ").unwrap();
        let numbered_reg = Regex::new(r"^\d+\. ").unwrap();
        let lines: Vec<String> = fs::read_to_string(&path).unwrap().lines().map(String::from).collect();
        let mut iter = lines.iter().peekable();
        loop {
            match iter.next() {
                Some(n) if n.starts_with("```") && !self.codeblock => {
                    self.write(Preprocessor::Text(self.current_joined()));
                    self.current.push(n.clone());
                    self.codeblock = true;
                },
                Some(n) if n.trim().ends_with("```") && self.codeblock => {
                    self.current.push(n.clone());
                    self.write(Preprocessor::CodeBlock(self.current_joined()));
                    self.codeblock = false;
                },
                Some(n) if n.starts_with("#") => {
                    let spl: Vec<&str> = n.split(" ").collect();
                    if spl[0].ends_with("#") {
                        self.write(Preprocessor::Text(self.current_joined()));
                        self.current.push(n.clone());
                        self.write(Preprocessor::Header(self.current_joined()));
                        self.current.clear();
                    } else {
                        self.current.push(n.clone());
                    }
                },
                Some(n) if n.starts_with("|") && !self.table &&!self.codeblock => {
                    match iter.peek() {
                        Some(l) if l.starts_with("|") && l.contains("---") => {
                            let spl = n.split("|");
                            let vec: Vec<String> = spl.clone().enumerate().filter(|&(i,_)| !(i == 0 || i == spl.clone().count()-1)).map(|s| s.1.trim()).map(String::from).collect::<Vec<_>>();
                            self.current_table.push(vec);
                            self.table = true;
                        },
                        _ => continue
                    }
                },
                Some(n) if n.starts_with("|") && self.table &&!self.codeblock => {
                    if n.contains("---") {continue};
                    let spl = n.split("|");
                    let vec: Vec<String> = spl.clone().enumerate().filter(|&(i,_)| !(i == 0 || i == spl.clone().count()-1)).map(|s| s.1.trim()).map(String::from).collect::<Vec<_>>();
                    self.current_table.push(vec);
                    match iter.peek() {
                        None => {
                            self.write(Preprocessor::Table(self.current_table.clone()));
                            self.table = false;
                        },
                        Some(l) if !l.starts_with("|") => {
                            self.write(Preprocessor::Table(self.current_table.clone()));
                            self.table = false;
                        },
                        _ => continue,
                    }
                },
                Some(n) if n.starts_with(">") &&!self.codeblock => {
                    if !self.callout {
                        self.write(Preprocessor::Text(self.current_joined()));
                        self.callout = true;
                    }
                    self.current.push(n.clone());
                    match iter.peek() {
                        Some(n) if !n.starts_with(">") => {
                            self.write(Preprocessor::Callout(self.current_joined()));
                            self.callout = false;
                        },
                        None => {
                            self.write(Preprocessor::Callout(self.current_joined()));
                            self.callout = false;
                        },
                        _ => continue
                    }
                },
                Some(n) if tasklist_reg.is_match(&n.trim()) && !self.codeblock => {
                    if !self.tasklist {
                        self.write(Preprocessor::Text(self.current_joined()));
                        self.tasklist = true;
                    }
                    self.current.push(String::from(n.trim_end()));
                    match iter.peek() {
                        Some(n) if !tasklist_reg.is_match(&n.trim()) => {
                            self.write(Preprocessor::TaskList(self.current.clone()));
                            self.tasklist = false;
                        },
                        None => {
                            self.write(Preprocessor::TaskList(self.current.clone()));
                            self.tasklist = false;
                        },
                        _ => continue
                    }
                },
                Some(n) if n.trim().starts_with("- ") && !self.codeblock => {
                    if !self.bulletlist {
                        self.write(Preprocessor::Text(self.current_joined()));
                        self.bulletlist = true;
                    }
                    self.current.push(String::from(n.trim_end()));
                    match iter.peek() {
                        Some(n) if !n.starts_with("- ") => {
                            self.write(Preprocessor::BulletList(self.current.clone()));
                            self.bulletlist = false;
                        },
                        None => {
                            self.write(Preprocessor::BulletList(self.current.clone()));
                            self.bulletlist = false;
                        },
                        _ => continue
                    }
                },
                Some(n) if numbered_reg.is_match(&n.trim()) && !self.codeblock => {
                    if !self.numberedlist {
                        self.write(Preprocessor::Text(self.current_joined()));
                        self.numberedlist = true;
                    }
                    self.current.push(String::from(n.trim_end()));
                    match iter.peek() {
                        Some(n) if !numbered_reg.is_match(&n.trim()) => {
                            self.write(Preprocessor::NumberedList(self.current.clone()));
                            self.numberedlist = false;
                        },
                        None => {
                            self.write(Preprocessor::NumberedList(self.current.clone()));
                            self.numberedlist = false;
                        },
                        _ => continue
                    }
                },
                Some(n) => {
                    self.current.push(n.clone());

                },
                None => {
                    self.write(Preprocessor::Text(self.current_joined()));
                    break;
                }
            }
        }
        self.node
    }
}

#[derive(Debug)]
struct PreprocessedNode {
    path: String,
    content: Vec<Preprocessor>
}

impl PreprocessedNode {
    fn parse(filename: String) -> Self {
        let parser = Parser::new();
        parser.parse(filename)
    }
}

pub struct Node {
    path: String,
    content: Vec<Section>,
}

impl Node {
    pub fn parse(filename: String) -> Self {
        let pre = PreprocessedNode::parse(filename);
        let content: Vec<Section> = pre.content.iter().map(Node::process).collect();
        Node { path: pre.path, content }
    }

    fn process(s: &Preprocessor) -> Section {
        match s {
            Preprocessor::Header(s) => {
                let count = s.chars().take_while(|c| *c=='#').count();
                let str = &s[count..];
                Section::Header(count as u32, String::from(str.trim()))
            },
            Preprocessor::Callout(s) => {
                Section::Callout(String::from(s[1..].trim()))
            },
            Preprocessor::CodeBlock(s) => {
                let s = s.trim_matches('`');
                let pl = s.split("\n").next().unwrap().to_string();
                let content = s.split("\n").collect::<Vec<&str>>()[1..].join("\n");
                Section::CodeBlock(pl, content)
            },
            Preprocessor::Table(t) => {
                Section::Table(t.clone())
            },
            Preprocessor::Text(t) => {
                Section::Text(t.clone())
            },
            Preprocessor::BulletList(s) => {
                let res: Vec<(u32,String)> = s.iter().map(|x| {
                    let tab = x.chars().take_while(|c| *c=='\t').count();
                    let text = x.trim().get(2..).unwrap().to_string();
                    (tab as u32,text)
                }).collect();
                Section::BulletList(res)
            },
            Preprocessor::TaskList(s) => {
                let res: Vec<(u32,bool,String)> = s.iter().map(|x| {
                    let tab = x.chars().take_while(|c| *c=='\t').count();
                    let bool = x.trim().chars().nth(3).unwrap() == 'x';
                    let text = x.trim().get(6..).unwrap().to_string();
                    (tab as u32,bool,text)
                }).collect();
                Section::TaskList(res)
            },
            Preprocessor::NumberedList(s) => {
                let res: Vec<(u32,String)> = s.iter().map(|x| {
                    let tab = x.chars().take_while(|c| *c=='\t').count();
                    let spl: Vec<&str> = x.trim().splitn(2," ").collect();
                    let text: String = spl[1].to_string();
                    (tab as u32,text)
                }).collect();
                Section::NumberedList(res)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODEBLOCK_PL: &str = "powershell";
    const CODEBLOCK: &str = "Get-ChildItem -Recurse\npeperober\nsalchichon\n";
    const TEXT: &str = "test file\ntesting\nhi";
    const HEADER1: &str = "Test";
    const HEADER2: &str = "Testing";
    const TABLE_0_0: &str = "Testcolumn1";
    const TABLE_1_0: &str = "Testcolumn2";
    const TABLE_0_1: &str = "result1";
    const TABLE_1_1: &str = "result2";
    const CALLOUT: &str = "Callout nice!\n>Pepechivich";
    const BULLETLIST1: &str = "Sandwich";
    const BULLETLIST2: &str = "Salami";
    const NUMBEREDLIST1: &str = "Pepeeeee";
    const NUMBEREDLIST2: &str = "Manoooooolo";
    const TASKLIST1: &str = "Pepe rocks";
    const TASKLIST2: &str = "chimichanga";

    #[test]
    fn simple_text() {
        let node = Node::parse(String::from("tests/simple.md"));
        assert_eq!(node.content,vec![Section::Text(String::from(TEXT))]);
    }
    #[test]
    fn code_block() {
        let node = Node::parse(String::from("tests/code.md"));
        assert_eq!(node.content,vec![Section::CodeBlock(String::from(CODEBLOCK_PL),String::from(CODEBLOCK))])
    }
    #[test]
    fn header() {
        let node = Node::parse(String::from("tests/header.md"));
        assert_eq!(node.content,vec![Section::Header(1,String::from(HEADER1))]);
    }
    #[test]
    fn multiple_headers() {
        let node = Node::parse(String::from("tests/multiple_headers.md"));
        assert_eq!(node.content,vec![Section::Header(1,String::from(HEADER1)),Section::Header(3,String::from(HEADER2))]);
    }
    #[test]
    fn table() {
        let table: Vec<Vec<String>> = vec![vec![String::from(TABLE_0_0),String::from(TABLE_1_0)],vec![String::from(TABLE_0_1),String::from(TABLE_1_1)]];
        let node = Node::parse(String::from("tests/table.md"));
        assert_eq!(node.content,vec![Section::Table(table)]);
    }
    #[test]
    fn callout() {
        let node = Node::parse(String::from("tests/callout.md"));
        assert_eq!(node.content,vec![Section::Callout(String::from(CALLOUT))]);
    }
    #[test]
    fn bulletlist() {
        let node = Node::parse(String::from("tests/bulletlist.md"));
        assert_eq!(node.content,vec![Section::BulletList(vec![(0,String::from(BULLETLIST1)),(0,String::from(BULLETLIST2))])]);
    }
    #[test]
    fn numberedlist() {
        let node = Node::parse(String::from("tests/numberedlist.md"));
        assert_eq!(node.content,vec![Section::NumberedList(vec![(0,String::from(NUMBEREDLIST1)),(0,String::from(NUMBEREDLIST2))])]);
    }
    #[test]
    fn tasklist() {
        let node = Node::parse(String::from("tests/tasklist.md"));
        assert_eq!(node.content,vec![Section::TaskList(vec![(0,false,String::from(TASKLIST1)),(0,true,String::from(TASKLIST2))])]);
    }
    #[test]
    fn multiple_things() {
        let node = Node::parse(String::from("tests/multiple.md"));
        let table: Vec<Vec<String>> = vec![vec![String::from(TABLE_0_0),String::from(TABLE_1_0)],vec![String::from(TABLE_0_1),String::from(TABLE_1_1)]];
        assert_eq!(node.content[0],Section::Header(1,String::from(HEADER1)));
        assert_eq!(node.content[1],Section::CodeBlock(String::from(CODEBLOCK_PL),String::from(CODEBLOCK)));
        assert_eq!(node.content[2],Section::Header(1,String::from(HEADER1)));
        assert_eq!(node.content[3],Section::Header(3,String::from(HEADER2)));
        assert_eq!(node.content[4],Section::Table(table));
        assert_eq!(node.content[5],Section::Text(String::from(TEXT)));
        assert_eq!(node.content[6],Section::Callout(String::from(CALLOUT)));
        assert_eq!(node.content[7],Section::BulletList(vec![(0,String::from(BULLETLIST1)),(0,String::from(BULLETLIST2))]));
    }
}
