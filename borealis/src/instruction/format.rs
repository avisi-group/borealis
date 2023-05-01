//! Instruction format string extraction

use {
    crate::genc::format::{InstructionFormat as GenCFormat, Segment, SegmentContent},
    common::{identifiable::unique_id, intern::InternedString},
    log::{trace, warn},
    num_bigint::Sign,
    once_cell::sync::Lazy,
    pretty_assertions::assert_eq,
    sail::{
        num::BigInt,
        sail_ast::{
            Expression, ExpressionAux, FunctionClause, Identifier, IdentifierAux, LValueExpression,
            LValueExpressionAux, Literal, LiteralAux, NumericExpression, NumericExpressionAux,
            Pattern, PatternAux, PatternMatchAux, TypArgAux, TypAux,
        },
    },
    std::{
        collections::{HashMap, LinkedList},
        fmt::{Debug, Display},
        ops::Range,
        sync::Mutex,
    },
};

static NAME_COUNTER: Lazy<NameCounter> = Lazy::new(NameCounter::new);

struct NameCounter(Mutex<HashMap<InternedString, u32>>);

impl NameCounter {
    /// Create a new empty instance
    fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    fn increment_count(&self, key: InternedString) -> u32 {
        let mut counters = self.0.lock().unwrap();

        let current = counters.get(&key).copied().unwrap_or(0);
        let last = counters.insert(key, current + 1);
        assert!(last == Some(current) || last.is_none());
        current
    }
}

/// Bit in an instruction format
#[derive(PartialEq, Eq)]
pub enum FormatBit {
    /// Fixed zero
    Zero,
    /// Fixed one
    One,
    /// Unknown bit
    Unknown,
}

impl FormatBit {
    fn is_unknown(&self) -> bool {
        match self {
            Self::Zero | Self::One => false,
            Self::Unknown => true,
        }
    }

    fn is_fixed(&self) -> bool {
        !self.is_unknown()
    }

    fn fixed_value(&self) -> u64 {
        match self {
            FormatBit::Zero => 0,
            FormatBit::One => 1,
            FormatBit::Unknown => panic!("unknown bit has no value"),
        }
    }
}

impl Debug for FormatBit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Zero => write!(f, "0"),
            Self::One => write!(f, "1"),
            Self::Unknown => write!(f, "x"),
        }
    }
}

/// Sequence of bits corresponding to the machine code representation of an instruction
#[derive(Debug)]
pub struct Format(Vec<FormatBit>);

impl Format {
    /// Create a new empty collection
    pub fn empty() -> Self {
        Self(vec![])
    }

    /// Appends a decode bit to the collection
    pub fn push(&mut self, bit: FormatBit) {
        self.0.push(bit)
    }

    /// Moves all bits in `other` into `self`
    pub fn append(&mut self, other: &mut Format) {
        self.0.append(&mut other.0);
    }

    /// Finish building a sequence of format bits
    pub fn finish(self) -> Vec<FormatBit> {
        self.0
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bit in &self.0 {
            write!(f, "{bit:?}")?;
        }
        Ok(())
    }
}

impl From<&Literal> for Format {
    fn from(literal: &Literal) -> Self {
        let LiteralAux::Bin(s) = &literal.inner else {
            panic!("Unexpected literal when decoding instruction format");
        };

        let mut decode_bits = Format::empty();
        for char in s.to_string().chars() {
            match char {
                '0' => decode_bits.push(FormatBit::Zero),
                '1' => decode_bits.push(FormatBit::One),
                c => panic!("Unexpected char {c:?} when decoding instruction format"),
            }
        }
        decode_bits
    }
}

/// Main function clause processing
pub fn process_decode_function_clause(
    funcl: &FunctionClause,
) -> (InternedString, InternedString, GenCFormat) {
    trace!(
        "Processing decode function clause @ {}",
        funcl.annotation.location
    );

    let (pat, body) = match &funcl.inner.pattern_match.inner {
        PatternMatchAux::Expression(pat, body) => (pat, body),
        PatternMatchAux::When(pat, _, body) => {
            warn!("Function clause has condition, ignoring...");
            (pat, body)
        }
    };

    let format_bits = extract_format(&pat.inner);

    let ExpressionAux::Block(expressions) = &*body.inner else {
        panic!("Body was not Block");
    };

    assert_eq!(expressions.len(), 2);

    assert!(is_see_assignment(expressions.front().unwrap()));

    let expressions = flatten_expression(expressions.back().unwrap());

    let mut named_ranges = expressions
        .iter()
        .take(expressions.len() - 1) // skip last expression
        .filter_map(expression_to_named_range)
        .collect::<Vec<_>>();

    let instruction_name = {
        let Some(Expression { inner, .. }) = expressions.last() else { panic!() };

        let ExpressionAux::Application(ident, ..) = &**inner else { panic!() };

        ident.as_interned()
    };

    // named ranges may not be contiguous, so fill any holes with padding ranges (these should not contain any unknown bits)
    {
        let mut padding_ranges = vec![];
        let mut last_end = 0;
        for (_, range) in &named_ranges {
            if range.start != last_end {
                // insert new range from last_end to range.start
                let padding = (
                    format!("padding{}", unique_id()).into(),
                    last_end..range.start,
                );
                warn!("inserting new padding range: {:?}", padding);
                padding_ranges.push(padding);
            }
            last_end = range.end;
        }

        if last_end != format_bits.len() {
            let padding = (
                format!("padding{}", unique_id()).into(),
                last_end..format_bits.len(),
            );
            warn!("inserting final padding range: {:?}", padding);
            padding_ranges.push(padding);
        }

        named_ranges.append(&mut padding_ranges);
        named_ranges.sort_by(|(_, a), (_, b)| a.start.cmp(&b.start));
    }

    trace!("named_ranges: {:?}", named_ranges);

    let homogenised_ranges = named_ranges
        .clone()
        .into_iter()
        .flat_map(|(n, r)| {
            let bits = &format_bits[r.clone()];

            // determine locations where bit kind changes
            let delta = bits
                .windows(2)
                .map(|window| window[0].is_unknown() != window[1].is_unknown())
                .collect::<Vec<_>>();

            // indexes where ranges must be split
            let split_indexes = delta
                .iter()
                .enumerate()
                .filter_map(|(i, x)| match x {
                    true => Some(i + 1),
                    false => None,
                })
                .collect::<Vec<_>>();

            let mut new_ranges = vec![];
            let mut current_start = r.start;
            for split_index in split_indexes {
                new_ranges.push(current_start..r.start + split_index);
                current_start = r.start + split_index;
            }
            new_ranges.push(current_start..r.end);

            new_ranges.into_iter().enumerate().map(move |(i, range)| {
                if i == 0 {
                    (n, range)
                } else {
                    (format!("{}_part{}", &n, i).into(), range)
                }
            })
        })
        .collect::<Vec<_>>();

    trace!("homogenised_ranges: {:?}", homogenised_ranges);

    let homogenised_bits = homogenised_ranges
        .into_iter()
        .map(|(n, r)| (n, &format_bits[r]))
        .collect::<Vec<_>>();

    trace!("homogenised_bits: {:?}", homogenised_bits);

    let inner = homogenised_bits
        .into_iter()
        .map(|(n, bits)| {
            let content = if bits.iter().all(FormatBit::is_unknown) {
                SegmentContent::Variable(n)
            } else if bits.iter().all(FormatBit::is_fixed) {
                SegmentContent::Constant(
                    bits.iter().fold(0, |acc, bit| acc << 1 | bit.fixed_value()),
                )
            } else {
                panic!();
            };

            Segment {
                content,
                length: bits.len(),
            }
        })
        .collect::<Vec<_>>();

    // all ARM instructions are 32 bits long
    assert_eq!(inner.iter().map(|s| s.length).sum::<usize>(), 32);

    let count = NAME_COUNTER.increment_count(instruction_name);

    let name = format!("{instruction_name}{count}").into();
    let format = GenCFormat(inner);
    trace!("{} genc format: {}", name, format);

    (name, instruction_name, format)
}

/// Flattens nested expressions
pub fn flatten_expression(expression: &Expression) -> Vec<Expression> {
    let mut expressions = vec![];

    let mut current_expression = expression.clone();

    loop {
        let (lhs, rhs, exp1) = match *current_expression.inner {
            ExpressionAux::Var(lhs, rhs, exp1) => (lhs, rhs, exp1),
            ExpressionAux::Application(..) => {
                expressions.push(current_expression);
                break;
            }
            _ => panic!("{}", <&'static str>::from(*current_expression.inner)),
        };

        current_expression.inner = Box::new(ExpressionAux::Assign(lhs.clone(), rhs.clone()));

        expressions.push(current_expression);

        let ExpressionAux::Block(expressions) = &*exp1.inner else { panic!() };

        match expressions.len() {
            0 => break,
            1 => {
                current_expression = expressions.front().unwrap().clone();
            }
            _ => panic!(),
        }
    }

    expressions
}

/// Extracts a sequence of format bits from function clause pattern.
///
/// For example the AST for the following pattern
///
/// ```sail
/// (0b1 @ _ : bits(1) @ 0b1110000 @ _ : bits(1) @ 0b1 @ _ : bits(5) @ 0b000000 @ _ : bits(5) @ 0b11111 as op_code
/// ```
///
/// to
///
/// ```text
/// 1x1110000x1xxxxx000000xxxxx11111
/// ```
pub fn extract_format(pattern_aux: &PatternAux) -> Vec<FormatBit> {
    let mut format_bits = Format::empty();

    let PatternAux::As(Pattern { inner, .. }, ident) = pattern_aux else {
        panic!();
    };

    assert_eq!(ident.to_string(), "Identifier(\"op_code\")".to_owned());

    let patterns = match &**inner {
        PatternAux::Literal(literal) => {
            trace!("op_code was a single literal!");
            return Format::from(literal).finish();
        }
        PatternAux::VectorConcat(patterns) => patterns,
        v => panic!(
            "expected VectorConcat or Literal, got {}",
            Into::<&'static str>::into(v)
        ),
    };

    for pattern in patterns {
        match &*pattern.inner {
            PatternAux::Literal(literal) => {
                format_bits.append(&mut Format::from(literal));
            }
            PatternAux::Type(typ, pat) => {
                match &*pat.inner {
                    PatternAux::Wildcard => (),
                    n => warn!("found non-wildcard: {}", Into::<&'static str>::into(n)),
                };

                if let TypAux::Application(
                    Identifier {
                        inner: IdentifierAux::Identifier(s),
                        ..
                    },
                    typargs,
                ) = &*typ.inner
                {
                    if s.to_string() != "bits" {
                        panic!();
                    }

                    if let TypArgAux::NExp(NumericExpression { inner: n, .. }) =
                        &typargs.front().unwrap().inner
                    {
                        if let NumericExpressionAux::Constant(sail::num::BigInt(big)) = &**n {
                            for _ in 0..big.to_u64_digits().1[0] {
                                format_bits.push(FormatBit::Unknown);
                            }
                        }
                    }
                }
            }
            pat => warn!("pattern aux was type {}", Into::<&'static str>::into(pat)),
        }
    }

    trace!("got format bits: {}", format_bits);

    format_bits.finish()
}

/// Converts a vector access expression to a range and name of the assigned variable.
///
/// For example the AST for the following expression
///
/// ```sail
/// Ra : bits(5) = op_code[14 .. 10];
/// ```
///
/// will be converted to
///
/// ```text
/// Some(("Ra", 10..15))
/// ```
pub fn expression_to_named_range(
    expression: &Expression,
) -> Option<(InternedString, Range<usize>)> {
    let ExpressionAux::Assign(LValueExpression { inner: lvalue, ..}, Expression { inner: expression_aux, .. }) = &*expression.inner else {
        panic!("ExpressionAux not an Assign");
    };

    let name = {
        let LValueExpressionAux::Cast(_, ident) = &**lvalue else {
            panic!("LValueExpression not a Cast");
        };

        ident.as_interned()
    };

    let range = match &**expression_aux {
        ExpressionAux::Vector(expressions) => {
            assert_eq!(expressions.len(), 1);
            bitvector_access_to_range(&expressions.front().unwrap().inner)
        }
        ExpressionAux::Application(ident, exps) => vector_subrange_to_range(ident, exps),
        exp => panic!(
            "Unexpected Expression variant {:?}",
            <&'static str>::from(exp)
        ),
    };

    Some((name, range))
}

/// Tests whether an expression is an assignment to `SEE`
pub fn is_see_assignment(expression: &Expression) -> bool {
    let ExpressionAux::Assign(LValueExpression { inner, ..}, _) = &*expression.inner else {
        return false;
    };

    let LValueExpressionAux::Identifier(Identifier {inner: IdentifierAux::Identifier(s),..}) = &**inner else {
        return false;
    };

    s.to_string() == "SEE"
}

/// Extracts a range from a `bitvector_access` function application
///
/// For example the AST for the following expression
///
/// ```sail
/// op_code[14 .. 10]
/// ```
///
/// will be converted to
///
/// ```text
/// 10..15
/// ```
pub fn bitvector_access_to_range(exp: &ExpressionAux) -> Range<usize> {
    let ExpressionAux::Application(ident, expressions) = exp else {
        panic!("not an application");
    };
    assert_eq!(ident.as_interned(), "bitvector_access_A".into());

    let mut iter = expressions.iter();

    // first expression should be the `op_code` identifier
    let ExpressionAux::Identifier(ident) = &*iter.next().unwrap().inner else {
        panic!("first expression was not identifier");
    };
    assert_eq!(ident.as_interned(), "op_code".into());

    let start = expression_to_usize(iter.next().unwrap());
    let end = start + 1;

    assert!(iter.next().is_none());

    start..end
}

/// Extracts a range from a `vector_subrange` function application
///
/// For example the AST for the following expression
///
/// ```sail
/// [op_code[30]]
/// ```
///
/// will be converted to
///
/// ```text
/// 30..31
/// ```
pub fn vector_subrange_to_range(
    ident: &Identifier,
    expressions: &LinkedList<Expression>,
) -> Range<usize> {
    assert_eq!(ident.as_interned(), "vector_subrange_A".into());

    let mut iter = expressions.iter();

    // first expression should be the `op_code` identifier
    let ExpressionAux::Identifier(ident) = &*iter.next().unwrap().inner else {
        panic!("first expression was not identifier");
    };
    assert_eq!(ident.as_interned(), "op_code".into());

    let end = expression_to_usize(iter.next().unwrap()) + 1;
    let start = expression_to_usize(iter.next().unwrap());

    assert!(iter.next().is_none());

    start..end
}

/// Extracts a `usize` from a Literal expression
pub fn expression_to_usize(expression: &Expression) -> usize {
    let ExpressionAux::Literal(Literal { inner: LiteralAux::Num(BigInt(bigint)), .. }) = &*expression.inner else {
        panic!("expression was not a num literal");
    };

    let (sign, digits) = bigint.to_u64_digits();

    assert!(sign != Sign::Minus);

    match digits.len() {
        0 => 0,
        1 => usize::try_from(digits[0]).unwrap(),
        _ => panic!("bigint {bigint:?} too large"),
    }
}
