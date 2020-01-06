use std::collections::HashSet;

#[derive(Debug)]
pub enum TransformKind {
    Substitute,
}

#[derive(Debug)]
pub struct TransformInstruction {
    pub kind: TransformKind,
    pub args: Vec<String>,
}

type ParseResult<'a, Output> = Result<(&'a str, Output), &'a str>;

trait Parser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output>;

    fn map<F, NewOutput>(self, map_fn: F) -> BoxedParser<'a, NewOutput>
    where
        Self: Sized + 'a,
        Output: 'a,
        NewOutput: 'a,
        F: Fn(Output) -> NewOutput + 'a,
    {
        BoxedParser::new(map(self, map_fn))
    }

    fn pred<F>(self, pred_fn: F) -> BoxedParser<'a, Output>
    where
        Self: Sized + 'a,
        Output: 'a,
        F: Fn(&Output) -> bool + 'a,
    {
        BoxedParser::new(pred(self, pred_fn))
    }
}

impl<'a, F, Output> Parser<'a, Output> for F
where
    F: Fn(&'a str) -> ParseResult<Output>,
{
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self(input)
    }
}

struct BoxedParser<'a, Output> {
    parser: Box<dyn Parser<'a, Output> + 'a>,
}

impl<'a, Output> BoxedParser<'a, Output> {
    fn new<P>(parser: P) -> Self
    where
        P: Parser<'a, Output> + 'a,
    {
        BoxedParser {
            parser: Box::new(parser),
        }
    }
}

impl<'a, Output> Parser<'a, Output> for BoxedParser<'a, Output> {
    fn parse(&self, input: &'a str) -> ParseResult<'a, Output> {
        self.parser.parse(input)
    }
}

fn literal<'a>(expected: &'static str) -> impl Parser<'a, ()> {
    move |input: &'a str| match input.get(0..expected.len()) {
        Some(next) if next == expected => Ok((&input[expected.len()..], ())),
        _ => Err(input),
    }
}

fn pair<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, (R1, R2)>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    move |input| {
        parser1.parse(input).and_then(|(next_input, result1)| {
            parser2
                .parse(next_input)
                .map(|(last_input, result2)| (last_input, (result1, result2)))
        })
    }
}

fn map<'a, P, F, A, B>(parser: P, map_fn: F) -> impl Parser<'a, B>
where
    P: Parser<'a, A>,
    F: Fn(A) -> B,
{
    move |input| {
        parser
            .parse(input)
            .map(|(next_input, result)| (next_input, map_fn(result)))
    }
}

fn left<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R1>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(left, _right)| left)
}

fn right<'a, P1, P2, R1, R2>(parser1: P1, parser2: P2) -> impl Parser<'a, R2>
where
    P1: Parser<'a, R1>,
    P2: Parser<'a, R2>,
{
    map(pair(parser1, parser2), |(_left, right)| right)
}

fn one_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        if let Ok((next_input, first_item)) = parser.parse(input) {
            input = next_input;
            result.push(first_item);
        } else {
            return Err(input);
        }

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}

fn zero_or_more<'a, P, A>(parser: P) -> impl Parser<'a, Vec<A>>
where
    P: Parser<'a, A>,
{
    move |mut input| {
        let mut result = Vec::new();

        while let Ok((next_input, next_item)) = parser.parse(input) {
            input = next_input;
            result.push(next_item);
        }

        Ok((input, result))
    }
}

fn any_char(input: &str) -> ParseResult<char> {
    match input.chars().next() {
        Some(next) => Ok((&input[next.len_utf8()..], next)),
        _ => Err(input),
    }
}

fn pred<'a, P, A, F>(parser: P, predicate: F) -> impl Parser<'a, A>
where
    P: Parser<'a, A>,
    F: Fn(&A) -> bool,
{
    move |input| {
        if let Ok((next_input, value)) = parser.parse(input) {
            if predicate(&value) {
                return Ok((next_input, value));
            }
        }
        Err(input)
    }
}

fn whitespace<'a>() -> impl Parser<'a, Vec<char>> {
    one_or_more(any_char.pred(|c| c.is_whitespace()))
}

fn join<'a, P>(parser: P) -> impl Parser<'a, String>
where
    P: Parser<'a, Vec<char>>,
{
    map(parser, |chars| chars.into_iter().collect())
}

fn char_blacklist(blacklist: &[char]) -> impl Parser<'_, char> {
    let set: HashSet<&char> = blacklist.iter().collect();
    move |input| {
        any_char(input).and_then(|(next_input, result)| {
            if result == '\\' {
                any_char(next_input)
            } else if set.contains(&result) {
                Err("")
            } else {
                Ok((next_input, result))
            }
        })
    }
}

fn transform_kind<'a>() -> impl Parser<'a, TransformKind> {
    literal("s").map(|()| TransformKind::Substitute)
}

fn transform_instruction<'a>() -> impl Parser<'a, TransformInstruction> {
    pair(
        transform_kind(),
        right(
            literal("/"),
            pair(
                join(zero_or_more(char_blacklist(&[' ', '/']))),
                right(
                    literal("/"),
                    left(
                        join(zero_or_more(char_blacklist(&[' ', '/']))),
                        literal("/"),
                    ),
                ),
            ),
        ),
    )
    .map(|(kind, args)| TransformInstruction {
        kind,
        args: vec![args.0, args.1],
    })
}

fn transform_instruction_list<'a>() -> impl Parser<'a, Vec<TransformInstruction>> {
    pair(
        transform_instruction(),
        zero_or_more(right(whitespace(), transform_instruction())),
    )
    .map(|(first, mut more)| {
        let mut list = vec![first];
        list.append(&mut more);
        list
    })
}

pub fn parse(input: &str) -> Result<Vec<TransformInstruction>, String> {
    let (remaining_input, result) = transform_instruction_list().parse(input)?;
    if !remaining_input.is_empty() {
        Err(format!("could not parse end of input {}", remaining_input))
    } else {
        Ok(result)
    }
}
