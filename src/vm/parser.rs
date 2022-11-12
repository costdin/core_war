use super::instructions::{Instruction, Modifier, OpCode, Operand, OperandMode};
use super::numeric::Numeric;
use std::collections::{HashMap, VecDeque};

fn get_labels<'a>(lines: &Vec<&'a str>) -> HashMap<&'a str, usize> {
    let mut labels = HashMap::<&str, usize>::new();

    for (ix, label) in lines
        .iter()
        .filter(|l| get_variable_definition(l).is_none()) // remove variables
        .enumerate()
        .map(|(ix, l)| (ix, l.split(':').collect::<Vec<_>>()))
        .filter(|(_, l)| l.len() == 2)
        .map(|(ix, l)| (ix, l[0].trim()))
    {
        labels.insert(label, ix);
    }

    labels
}

fn get_variables<'a>(
    lines: &Vec<&'a str>,
    labels: &HashMap<&str, usize>,
) -> Result<HashMap<&'a str, String>, String> {
    let mut variables = HashMap::<&str, &str>::new();
    let mut keys = Vec::<&str>::new();

    for (name, value) in lines.iter().filter_map(|l| get_variable_definition(l)) {
        variables.insert(name, value);
        keys.push(name);
    }

    let r = keys
        .iter()
        .map(|k| (k, expand_variable(variables[k], labels, &mut variables)))
        .map(|(k, v)| {
            if let Ok(value) = v {
                Ok((*k, value))
            } else {
                Err("e".to_string())
            }
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .collect();

    Ok(r)
}

fn expand_variable<'a>(
    value: &'a str,
    labels: &HashMap<&str, usize>,
    variables: &mut HashMap<&'a str, &'a str>,
) -> Result<String, String> {
    let mut result = String::new();

    for token in split_into_tokens(value) {
        if token.len() == 1 && TOKEN_BREAKER.contains(&token.chars().nth(0).unwrap())
            || token.chars().all(|c| c.is_numeric())
            || labels.contains_key(token)
        {
            result += token;
        } else {
            // remove and re-add token to prevent cyclic references
            result += &match variables.remove(token) {
                Some(t) => {
                    let r = expand_variable(t, labels, variables)?;
                    variables.insert(token, t);

                    r
                }
                None => return Err(format!("Invalid token {}", token)),
            }
        }
    }

    Ok(result)
}

fn get_variable_definition(line: &str) -> Option<(&str, &str)> {
    let v = line.split_whitespace().collect::<Vec<_>>();
    if v.len() == 3 && v[1] == "equ" {
        Some((v[0].trim(), v[2].trim()))
    } else {
        None
    }
}

pub fn parse<const CORE_SIZE: usize>(input: String) -> Result<Vec<Instruction<CORE_SIZE>>, String> {
    let lines = input
        .split(['\n', '\r'].as_ref())
        .map(|s| s.split(';').next().unwrap().trim()) // remove comments
        .filter(|l| !l.is_empty()) // remove empty rows
        .collect::<Vec<_>>();

    let mut result = vec![];

    let labels = get_labels(&lines);
    let variables = get_variables(&lines, &labels)?;

    for (ix, line) in lines
        .iter()
        .filter(|l| get_variable_definition(l).is_none()) // remove variables
        .enumerate()
    {
        let c: Vec<&str> = line.split([':'].as_ref()).filter(|l| l != &"").collect();

        let tl = if c.len() == 2 { c[1].trim() } else { line };

        let l: Vec<&str> = tl
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|l| l != &"")
            .collect();

        if l.len() != 3 {
            return Err(format!("Invalid line {}", tl));
        }

        let (op_code, modifier_opt) = parse_op_code(l[0])?;
        let a_operand = parse_operand(l[1], ix, &labels, &variables)?;
        let b_operand = parse_operand(l[2], ix, &labels, &variables)?;

        let modifier = match modifier_opt {
            Some(m) => m,
            None => implicit_modifier(&op_code, &a_operand, &b_operand),
        };

        result.push(Instruction {
            op: op_code,
            modifier,
            a_operand,
            b_operand,
        })
    }

    Ok(result)
}

fn implicit_modifier<const CORE_SIZE: usize>(
    op_code: &OpCode,
    a_operand: &Operand<CORE_SIZE>,
    b_operand: &Operand<CORE_SIZE>,
) -> Modifier {
    match op_code {
        OpCode::Dat => Modifier::F,
        OpCode::Mov | OpCode::Cmp => match (a_operand.mode, b_operand.mode) {
            (OperandMode::Immediate, _) => Modifier::AB,
            (_, OperandMode::Immediate) => Modifier::B,
            _ => Modifier::I,
        },
        OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div | OpCode::Mod => {
            match (a_operand.mode, b_operand.mode) {
                (OperandMode::Immediate, _) => Modifier::AB,
                (_, OperandMode::Immediate) => Modifier::B,
                _ => Modifier::F,
            }
        }
        OpCode::Slt => match (a_operand.mode, b_operand.mode) {
            (OperandMode::Immediate, _) => Modifier::AB,
            _ => Modifier::B,
        },
        _ => Modifier::B,
    }
}

fn parse_op_code(s: &str) -> Result<(OpCode, Option<Modifier>), String> {
    let tokens: Vec<&str> = s.split('.').collect();
    let op_code_string = tokens[0];
    let modifier_string = if tokens.len() == 2 {
        Some(tokens[1].to_lowercase())
    } else {
        None
    };

    let op_code = match op_code_string.to_lowercase().as_str() {
        "dat" => OpCode::Dat,
        "mov" => OpCode::Mov,
        "add" => OpCode::Add,
        "sub" => OpCode::Sub,
        "mul" => OpCode::Mul,
        "div" => OpCode::Div,
        "mod" => OpCode::Mod,
        "jmp" => OpCode::Jmp,
        "jmz" => OpCode::Jmz,
        "jmn" => OpCode::Jmn,
        "djn" => OpCode::Djn,
        "cmp" => OpCode::Cmp,
        "slt" => OpCode::Slt,
        "spl" => OpCode::Spl,
        _ => return Err(format!("Invalid OpCode: {}", s)),
    };

    let modifier = match modifier_string {
        None => None,
        Some(s) => {
            let m = match s.as_str() {
                "a" => Modifier::A,
                "b" => Modifier::B,
                "ab" => Modifier::AB,
                "ba" => Modifier::BA,
                "f" => Modifier::F,
                "x" => Modifier::X,
                "i" => Modifier::I,
                _ => return Err(format!("Invalid modifier: {}", s)),
            };
            Some(m)
        }
    };

    Ok((op_code, modifier))
}

fn parse_operand<const CORE_SIZE: usize>(
    s: &str,
    current_index: usize,
    labels: &HashMap<&str, usize>,
    variables: &HashMap<&str, String>,
) -> Result<Operand<CORE_SIZE>, String> {
    let first_char = s.chars().next().unwrap();
    let (operand_mode, start_ix) = match first_char {
        '#' => (OperandMode::Immediate, 1),
        '$' => (OperandMode::Direct, 1),
        '@' => (OperandMode::Indirect, 1),
        '<' => (OperandMode::Decrement, 1),
        '>' => (OperandMode::Increment, 1),
        _ => (OperandMode::Direct, 0),
    };

    let op_value = evaluate_operand::<CORE_SIZE>(&s[start_ix..], labels, variables, current_index)?;

    let pointer = Numeric::from(op_value as usize);

    Ok(Operand {
        pointer,
        mode: operand_mode,
    })
}

fn evaluate_operand<const CORE_SIZE: usize>(
    value: &str,
    labels: &HashMap<&str, usize>,
    variables: &HashMap<&str, String>,
    current_index: usize,
) -> Result<usize, String> {
    let res = match value.parse::<i128>() {
        Ok(n) => to_core_size::<CORE_SIZE>(n),
        _ if labels.contains_key(&value) => labels[value] + CORE_SIZE - current_index,
        _ => {
            let mut mega_stack = Vec::new();
            let mut value_stack = VecDeque::new();
            let mut op_stack = vec![];
            for token in
                operand_to_expression_tokens::<CORE_SIZE>(value, labels, variables, current_index)?
            {
                match token {
                    ExpressionToken::Value(v) => value_stack.push_back(ExpressionTree::Leaf(v)),
                    ExpressionToken::Operator(op) => op_stack.push(op),
                    ExpressionToken::OpenParenthesis => {
                        mega_stack.push((value_stack, op_stack));
                        value_stack = VecDeque::new();
                        op_stack = vec![];
                    }
                    ExpressionToken::CloseParenthesis => {
                        let value = process_final_elements_in_stack(value_stack, op_stack)?;

                        let (vs, os) = mega_stack.pop().unwrap();
                        value_stack = vs;
                        value_stack.push_back(value);
                        op_stack = os;
                    }
                };

                if op_stack.len() == 2 && value_stack.len() == 3 {
                    let last_op = op_stack.pop().unwrap();
                    let first_op = op_stack.pop().unwrap();

                    if last_op.takes_precedence(first_op) {
                        let right = value_stack.pop_back().unwrap();
                        let left = value_stack.pop_back().unwrap();

                        op_stack.push(first_op);
                        value_stack.push_back(ExpressionTree::create_node(left, right, last_op));
                    } else {
                        let left = value_stack.pop_front().unwrap();
                        let right = value_stack.pop_front().unwrap();

                        op_stack.push(last_op);
                        value_stack.push_front(ExpressionTree::create_node(left, right, first_op));
                    }
                }
            }

            process_final_elements_in_stack(value_stack, op_stack)?.evaluate()
        }
    };

    Ok(res)
}

fn process_final_elements_in_stack(
    mut value_stack: VecDeque<ExpressionTree>,
    mut op_stack: Vec<ExpressionOperator>,
) -> Result<ExpressionTree, String> {
    match (
        value_stack.pop_front(),
        value_stack.pop_front(),
        op_stack.pop(),
    ) {
        (Some(left), Some(right), Some(op)) => Ok(ExpressionTree::create_node(left, right, op)),
        (Some(v), None, None) => Ok(v),
        _ => Err("WAT?".to_string()),
    }
}

fn to_core_size<const CORE_SIZE: usize>(n: i128) -> usize {
    if n > 0 {
        n as usize
    } else {
        ((n % CORE_SIZE as i128) + CORE_SIZE as i128) as usize
    }
}

fn operand_to_expression_tokens<const CORE_SIZE: usize>(
    operand_value: &str,
    labels: &HashMap<&str, usize>,
    variables: &HashMap<&str, String>,
    current_index: usize,
) -> Result<Vec<ExpressionToken>, String> {
    let splitted = split_into_tokens(operand_value);

    let rs = splitted
        .iter()
        .map(|t| match ExpressionToken::parse::<CORE_SIZE>(t) {
            Ok(v) => Ok(vec![v]),
            _ => {
                if let Some(u) = labels.get(t) {
                    Ok(vec![ExpressionToken::Value(*u + CORE_SIZE - current_index)])
                } else if let Some(u) = variables.get(t) {
                    operand_to_expression_tokens::<CORE_SIZE>(u, labels, variables, current_index)
                } else {
                    Err(format!("Invalid token {}", t))
                }
            }
        });

    let mut result = vec![];
    for v in rs {
        result.append(&mut v?);
    }

    Ok(result)
}

fn split_into_tokens(s: &str) -> Vec<&str> {
    let mut res = vec![];
    let mut start = 0;

    for (ix, c) in s.char_indices() {
        if TOKEN_BREAKER.contains(&c) {
            if start != ix {
                res.push(&s[start..(ix)])
            }
            res.push(&s[ix..(ix + 1)]);
            start = ix + 1;
        }
    }

    // add last element
    if start < s.len() {
        res.push(&s[start..]);
    }

    res
}

static TOKEN_BREAKER: &'static [char] = &['+', '-', '*', '/', '%', '(', ')'];

#[derive(Debug)]
enum ExpressionToken {
    Operator(ExpressionOperator),
    Value(usize),
    OpenParenthesis,
    CloseParenthesis,
}

impl ExpressionToken {
    fn parse<const CORE_SIZE: usize>(s: &str) -> Result<ExpressionToken, String> {
        if let Ok(n) = s.parse::<i128>() {
            return Ok(ExpressionToken::Value(to_core_size::<CORE_SIZE>(n)));
        }

        if let Ok(o) = ExpressionOperator::parse(s) {
            return Ok(ExpressionToken::Operator(o));
        }

        match s {
            "(" => Ok(ExpressionToken::OpenParenthesis),
            ")" => Ok(ExpressionToken::CloseParenthesis),
            _ => Err(format!("Can't parse {}", s)),
        }
    }
}

#[derive(Debug)]
enum ExpressionTree {
    Leaf(usize),
    Node(Box<ExpressionTree>, Box<ExpressionTree>, ExpressionOperator),
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum ExpressionOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl ExpressionTree {
    fn evaluate(&self) -> usize {
        match self {
            ExpressionTree::Leaf(v) => *v,
            ExpressionTree::Node(left, right, operand) => match operand {
                ExpressionOperator::Add => left.evaluate() + right.evaluate(),
                ExpressionOperator::Sub => left.evaluate() - right.evaluate(),
                ExpressionOperator::Mul => left.evaluate() * right.evaluate(),
                ExpressionOperator::Div => left.evaluate() / right.evaluate(),
                ExpressionOperator::Mod => left.evaluate() % right.evaluate(),
            },
        }
    }

    fn create_node(
        left: ExpressionTree,
        right: ExpressionTree,
        op: ExpressionOperator,
    ) -> ExpressionTree {
        ExpressionTree::Leaf(ExpressionTree::Node(Box::new(left), Box::new(right), op).evaluate())
    }
}

impl ExpressionOperator {
    fn parse(s: &str) -> Result<ExpressionOperator, String> {
        let r = match s {
            "+" => ExpressionOperator::Add,
            "-" => ExpressionOperator::Sub,
            "*" => ExpressionOperator::Mul,
            "/" => ExpressionOperator::Div,
            "%" => ExpressionOperator::Mod,
            v => return Err(format!("Invalid operand {}", v)),
        };

        return Ok(r);
    }

    fn takes_precedence(self, other: ExpressionOperator) -> bool {
        return (self == ExpressionOperator::Mul
            || self == ExpressionOperator::Div
            || self == ExpressionOperator::Mod)
            && (other == ExpressionOperator::Add || other == ExpressionOperator::Sub);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_value() {
        let result = evaluate_operand::<8000>("99", &HashMap::new(), &HashMap::new(), 0).unwrap();

        assert_eq!(99, result);
    }

    #[test]
    fn simple_expression() {
        let result =
            evaluate_operand::<8000>("10*12+7", &HashMap::new(), &HashMap::new(), 0).unwrap();

        assert_eq!(127, result);
    }

    #[test]
    fn operator_precedence() {
        let result =
            evaluate_operand::<8000>("5+10*12+7", &HashMap::new(), &HashMap::new(), 0).unwrap();

        assert_eq!(132, result);
    }

    #[test]
    fn operator_precedence2() {
        let result = evaluate_operand::<8000>(
            "5+10*12+7/2+12*4-1*4*4*2",
            &HashMap::new(),
            &HashMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(144, result);
    }

    #[test]
    fn split_token_test() {
        let result = split_into_tokens("(5+10)*(12+7)/(2+12)*(4-1)*4*4*2");

        assert_eq!(29, result.len());
    }

    #[test]
    fn split_token_that_end_with_parentesis_test() {
        let result = split_into_tokens("((5+10)*(12+7)/(2+12)*(4-1)*4*4*2)");

        assert_eq!(31, result.len());
    }

    #[test]
    fn parantheses() {
        let result = evaluate_operand::<8000>(
            "(5+10)*(12+7)/(2+12)*(4-1)*4*4*2",
            &HashMap::new(),
            &HashMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(1920, result);
    }

    #[test]
    fn nested_parantheses() {
        let result = evaluate_operand::<8000>(
            "((1+5)*(1+2*(3+2)))*(12+7)/(2+12)*(4-1)*4*4+1*2",
            &HashMap::new(),
            &HashMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(4274, result);
    }

    #[test]
    fn nested_parantheses2() {
        let result = evaluate_operand::<8000>(
            "((1+5)+(1+2*(3+2)))*(12+7)/(2+12)*(4-1)*4*4+1*2",
            &HashMap::new(),
            &HashMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(1106, result);
    }

    #[test]
    fn parse_800() {
        test_parse::<800>();
    }

    #[test]
    fn parse_1000() {
        test_parse::<1000>();
    }

    #[test]
    fn parse_2000() {
        test_parse::<2000>();
    }

    #[test]
    fn parse_8000() {
        test_parse::<8000>();
    }

    #[test]
    fn parse_80000() {
        test_parse::<80000>();
    }

    fn test_parse<const CORE_SIZE: usize>() {
        let code = "
            lozzero equ 66
            mov 6, -1 ; i babbari
        ;borgo pio
            spl 6, <-3
            spl 7, <-4
    gaga:add #4, 3
            mov 2, @2
            jmp gaga, 0
            dat <3, <3
            spl 0, <-9
            dat <-10, <1
            spl imp, 0
            mov 0, -20,
            mov 1, -22,
            jmp -23, 0
    imp: spl 0, lozzero
            mov 0, 1"
            .to_string();

        let res = parse::<CORE_SIZE>(code).unwrap();

        assert_eq!(15, res.len());
        assert_eq!(OpCode::Mov, res[0].op);
        assert_eq!(Modifier::I, res[0].modifier);
        assert_eq!(6, res[0].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[0].a_operand.mode);
        assert_eq!(CORE_SIZE - 1, res[0].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[0].b_operand.mode);

        assert_eq!(OpCode::Spl, res[1].op);
        assert_eq!(Modifier::B, res[1].modifier);
        assert_eq!(6, res[1].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[1].a_operand.mode);
        assert_eq!(CORE_SIZE - 3, res[1].b_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[1].b_operand.mode);

        assert_eq!(OpCode::Spl, res[2].op);
        assert_eq!(Modifier::B, res[2].modifier);
        assert_eq!(7, res[2].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[2].a_operand.mode);
        assert_eq!(CORE_SIZE - 4, res[2].b_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[2].b_operand.mode);

        assert_eq!(OpCode::Add, res[3].op);
        assert_eq!(Modifier::AB, res[3].modifier);
        assert_eq!(4, res[3].a_operand.pointer.value);
        assert_eq!(OperandMode::Immediate, res[3].a_operand.mode);
        assert_eq!(3, res[3].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[3].b_operand.mode);

        assert_eq!(OpCode::Mov, res[4].op);
        assert_eq!(Modifier::I, res[4].modifier);
        assert_eq!(2, res[4].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[4].a_operand.mode);
        assert_eq!(2, res[4].b_operand.pointer.value);
        assert_eq!(OperandMode::Indirect, res[4].b_operand.mode);

        assert_eq!(OpCode::Jmp, res[5].op);
        assert_eq!(Modifier::B, res[5].modifier);
        assert_eq!(CORE_SIZE - 2, res[5].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[5].a_operand.mode);
        assert_eq!(0, res[5].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[5].b_operand.mode);

        assert_eq!(OpCode::Dat, res[6].op);
        assert_eq!(Modifier::F, res[6].modifier);
        assert_eq!(3, res[6].a_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[6].a_operand.mode);
        assert_eq!(3, res[6].b_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[6].b_operand.mode);

        assert_eq!(OpCode::Spl, res[7].op);
        assert_eq!(Modifier::B, res[7].modifier);
        assert_eq!(0, res[7].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[7].a_operand.mode);
        assert_eq!(CORE_SIZE - 9, res[7].b_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[7].b_operand.mode);

        assert_eq!(OpCode::Dat, res[8].op);
        assert_eq!(Modifier::F, res[8].modifier);
        assert_eq!(CORE_SIZE - 10, res[8].a_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[8].a_operand.mode);
        assert_eq!(1, res[8].b_operand.pointer.value);
        assert_eq!(OperandMode::Decrement, res[8].b_operand.mode);

        assert_eq!(OpCode::Spl, res[9].op);
        assert_eq!(Modifier::B, res[9].modifier);
        assert_eq!(4, res[9].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[9].a_operand.mode);
        assert_eq!(0, res[9].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[9].b_operand.mode);

        assert_eq!(OpCode::Mov, res[10].op);
        assert_eq!(Modifier::I, res[10].modifier);
        assert_eq!(0, res[10].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[10].a_operand.mode);
        assert_eq!(CORE_SIZE - 20, res[10].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[10].b_operand.mode);

        assert_eq!(OpCode::Mov, res[11].op);
        assert_eq!(Modifier::I, res[11].modifier);
        assert_eq!(1, res[11].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[11].a_operand.mode);
        assert_eq!(CORE_SIZE - 22, res[11].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[11].b_operand.mode);

        assert_eq!(OpCode::Jmp, res[12].op);
        assert_eq!(Modifier::B, res[12].modifier);
        assert_eq!(CORE_SIZE - 23, res[12].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[12].a_operand.mode);
        assert_eq!(0, res[12].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[12].b_operand.mode);

        assert_eq!(OpCode::Spl, res[13].op);
        assert_eq!(Modifier::B, res[13].modifier);
        assert_eq!(0, res[13].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[13].a_operand.mode);
        assert_eq!(66, res[13].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[13].b_operand.mode);

        assert_eq!(OpCode::Mov, res[14].op);
        assert_eq!(Modifier::I, res[14].modifier);
        assert_eq!(0, res[14].a_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[14].a_operand.mode);
        assert_eq!(1, res[14].b_operand.pointer.value);
        assert_eq!(OperandMode::Direct, res[14].b_operand.mode);
    }
}
