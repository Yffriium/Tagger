use std::{iter::Peekable, slice::Iter};

use crate::file::{Img, Tag, TagIdx};

#[derive(Debug)]
enum Token {
    Tag(String),
    Not,
    Or,
    LParen,
    RParen,
}

fn tokenize(input: String) -> Option<Vec<Token>> {
    let mut chars = input.chars().peekable();

    let mut vec: Vec<Token> = Vec::new();

    loop {
        let next_char = chars.next();

        match next_char {
            None => break,
            Some('-') => vec.push(Token::Not),
            Some(',') => vec.push(Token::Or),
            Some('(') => vec.push(Token::LParen),
            Some(')') => vec.push(Token::RParen),
            Some(' ') => {} // skip this
            Some(c) => {
                let mut building_string = String::from(c);
                'string: loop {
                    let next_char = chars.peek();
                    match next_char {
                        None => break 'string,
                        Some('-') => break 'string,
                        Some(',') => break 'string,
                        Some('(') => break 'string,
                        Some(')') => break 'string,
                        Some(' ') => break 'string,
                        Some(w) => {
                            building_string.push(*w);
                            chars.next();
                        }
                    }
                }
                vec.push(Token::Tag(building_string))
            }
        }
    }

    if vec.is_empty() {
        return None;
    }
    Some(vec)
}

#[derive(Debug)]
enum Expr {
    Tag(TagIdx),
    Not(Box<Expr>),
    Or(Vec<Expr>),
    And(Vec<Expr>),
}

fn parse(tokens: Vec<Token>, tags_list: &Vec<Tag>) -> Result<Expr, String> {
    let mut parser = Parser {
        iter: tokens.iter().peekable(),
        tags_list,
    };

    parser.and_expr()
}

///
/// Sees if we can fast filter on the expression.
/// We can fast filter on expressions that are just one AND, containing
/// only tags and nots of tags.
fn can_fast_filter_on(expr: &Expr) -> bool {
    match expr {
        Expr::Tag(_) => true,
        Expr::Not(other) => matches!(**other, Expr::Tag(_)),
        Expr::Or(_) => false,
        Expr::And(exprs) => {
            for e in exprs {
                match e {
                    Expr::Tag(_) => {}
                    Expr::Not(other) => match **other {
                        Expr::Tag(_) => {}
                        _ => return false,
                    },
                    Expr::Or(_) => return false,
                    Expr::And(_) => return false,
                }
            }
            true
        }
    }
}

fn fast_filter(
    banned_tidxs: Option<Vec<TagIdx>>,
    mandatory_tidxs: Option<Vec<TagIdx>>,
    images: &[Img],
) -> Vec<usize> {
    let mut non_outlawed: Vec<usize>;
    let mut has_mandatory: Vec<usize>;

    match banned_tidxs {
        Some(banned) => {
            non_outlawed = Vec::new();

            for (idx, img) in images.iter().enumerate() {
                match img.tags.as_ref() {
                    Some(tags) => {
                        let mut fine: bool = true;
                        'tidx_loop: for tidx in banned.iter() {
                            if tags.contains(tidx) {
                                fine = false;
                                break 'tidx_loop;
                            }
                        }

                        if fine {
                            non_outlawed.push(idx);
                        }
                    }
                    None => non_outlawed.push(idx),
                }
            }
        }
        None => {
            non_outlawed = (0..images.len()).collect();
        }
    }

    match mandatory_tidxs {
        Some(mandatory) => {
            has_mandatory = Vec::new();
            for idx in non_outlawed {
                let img = &images[idx];
                if let Some(tags) = img.tags.as_ref() {
                    let mut fine: bool = true;
                    'tidx_loop: for tidx in mandatory.iter() {
                        if !tags.contains(tidx) {
                            fine = false;
                            break 'tidx_loop;
                        }
                    }

                    if fine {
                        has_mandatory.push(idx);
                    }
                }
            }
        }
        None => has_mandatory = non_outlawed,
    }

    has_mandatory
}

fn filter_to_expr(expr: Expr, images: &[Img]) -> Vec<usize> {
    let can_fast_filter: bool = can_fast_filter_on(&expr);

    if can_fast_filter {
        // this is the easier case
        let mut outlawed: Vec<TagIdx> = Vec::new();
        let mut mandatory: Vec<TagIdx> = Vec::new();

        match expr {
            Expr::Tag(t) => mandatory.push(t),
            Expr::Not(expr) => match *expr {
                Expr::Tag(t) => outlawed.push(t),
                _ => panic!("Can't get here! We said we could fast filter."),
            },
            Expr::Or(_) => panic!("Shouldn't get here. can fast filter, so no or."),
            Expr::And(exprs) => {
                for e in exprs {
                    match e {
                        Expr::Tag(t) => mandatory.push(t),
                        Expr::Not(other) => match *other {
                            Expr::Tag(t) => outlawed.push(t),
                            _ => panic!("Can't get here, said we had fast filter."),
                        },
                        Expr::Or(_) => panic!("Can't have OR in AND with fast filter."),
                        Expr::And(_) => panic!("Can't have AND in AND with fast filter."),
                    }
                }
            }
        }

        let outlawed_option = match outlawed.len() {
            0 => None,
            _ => Some(outlawed),
        };
        let mandatory_option = match mandatory.len() {
            0 => None,
            _ => Some(mandatory),
        };

        return fast_filter(outlawed_option, mandatory_option, images);
    }

    todo!()

    // this is the general, harder case
}

///
/// Provide a string (of the standard tag search format) and the images.
/// It will give back a vector detailing the indices of the images to filter to.
pub fn filter_to_string(
    input: String,
    tags: &Vec<Tag>,
    images: &[Img],
) -> Result<Vec<usize>, String> {
    let tokens = match tokenize(input) {
        Some(t) => t,
        None => {
            return Ok((0..images.len()).collect()); // everything, no statement
        }
    };
    println!("Got these tokens: {:?}", tokens);
    let expr = parse(tokens, tags)?;

    println!("Got this expr: {:?}", expr);

    Ok(filter_to_expr(expr, images))
}

struct Parser<'a> {
    iter: Peekable<Iter<'a, Token>>,
    tags_list: &'a Vec<Tag>,
}

impl<'a> Parser<'a> {
    fn and_expr(&mut self) -> Result<Expr, String> {
        let mut and_vec: Vec<Expr> = Vec::new();
        and_vec.push(self.or_expr()?);
        loop {
            match self.iter.peek() {
                None => break,
                Some(Token::RParen) => break,
                Some(_) => {
                    and_vec.push(self.or_expr()?);
                }
            }
        }
        if and_vec.len() == 1 {
            return Ok(and_vec.pop().unwrap());
        }
        Ok(Expr::And(and_vec))
    }

    fn or_expr(&mut self) -> Result<Expr, String> {
        let mut or_vec: Vec<Expr> = Vec::new();
        or_vec.push(self.not_expr()?);
        while let Some(Token::Or) = self.iter.peek() {
            self.iter.next();
            or_vec.push(self.not_expr()?);
        }
        // don't make OR if just the one
        if or_vec.len() == 1 {
            return Ok(or_vec.pop().unwrap());
        }
        Ok(Expr::Or(or_vec))
    }

    fn not_expr(&mut self) -> Result<Expr, String> {
        match self.iter.peek() {
            Some(Token::Not) => {
                self.iter.next();
                Ok(Expr::Not(Box::new(self.primary_expr()?)))
            }
            _ => self.primary_expr(),
        }
    }
    fn primary_expr(&mut self) -> Result<Expr, String> {
        match self.iter.peek() {
            Some(Token::LParen) => {
                self.iter.next();
                let inside_expr = self.and_expr()?;
                match self.iter.peek() {
                    Some(Token::RParen) => {
                        self.iter.next();
                        Ok(inside_expr)
                    }
                    _ => Err(String::from("Could not parse. Invalid parens.")),
                }
            }
            Some(Token::Tag(v)) => {
                self.iter.next();
                // look up
                let idx_opt = self.tags_list.iter().position(|tag| tag.name == *v);
                match idx_opt {
                    None => Err(format!("No tag of name {} exists.", *v)),
                    Some(i) => Ok(Expr::Tag(i as TagIdx)),
                }
            }
            _ => Err(String::from("Invalid search expression. Could not parse.")),
        }
    }
}
