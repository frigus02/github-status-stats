type Input = string;

type Parser<TResult> = (input: Input) => [TResult, Input];

export type TransformType = "s";

export interface TransformInstruction {
  type: TransformType;
  args: string[];
}

const literal = <TResult extends string>(
  literal: TResult
): Parser<TResult> => input => {
  if (!input.startsWith(literal)) {
    throw new Error(`expected "${literal}" at position 0 of "${input}"`);
  }

  return [literal, input.substr(literal.length)];
};

const charBlacklist = (blacklistedChars: string[]): Parser<string> => {
  const blacklist = new Set(blacklistedChars);
  return input => {
    if (input.length < 1) {
      throw new Error("unexpected end of input");
    }

    if (input.length >= 2 && input[0] === "\\") {
      return [input[1], input.substr(2)];
    }

    if (blacklist.has(input[0])) {
      throw new Error(
        `chars "${blacklist}" not allowed at position 0 of "${input}"`
      );
    }

    return [input[0], input.substr(1)];
  };
};

const pair = <TResultA, TResultB>(
  parserA: Parser<TResultA>,
  parserB: Parser<TResultB>
): Parser<[TResultA, TResultB]> => input => {
  let resultA: TResultA, resultB: TResultB;
  [resultA, input] = parserA(input);
  [resultB, input] = parserB(input);
  return [[resultA, resultB], input];
};

const zeroOrMany = <TResult>(
  parser: Parser<TResult>
): Parser<TResult[]> => input => {
  const results: TResult[] = [];
  let result: TResult;
  while (input.length > 0) {
    try {
      [result, input] = parser(input);
      results.push(result);
    } catch (e) {
      break;
    }
  }

  return [results, input];
};

const oneOrMany = <TResult>(
  parser: Parser<TResult>
): Parser<TResult[]> => input => {
  const results: TResult[] = [];
  let result: TResult;

  [result, input] = parser(input);
  results.push(result);

  while (input.length > 0) {
    try {
      [result, input] = parser(input);
      results.push(result);
    } catch (e) {
      break;
    }
  }

  return [results, input];
};

const map = <TResultIn, TResultOut>(fn: (result: TResultIn) => TResultOut) => (
  parser: Parser<TResultIn>
): Parser<TResultOut> => input => {
  let result: TResultIn;
  [result, input] = parser(input);
  return [fn(result), input];
};

const join = map((result: string[]) => result.join(""));

const left = <TResultA, TResultB>(
  parserA: Parser<TResultA>,
  parserB: Parser<TResultB>
): Parser<TResultA> =>
  map(([resultA, _]: [TResultA, TResultB]) => resultA)(pair(parserA, parserB));

const right = <TResultA, TResultB>(
  parserA: Parser<TResultA>,
  parserB: Parser<TResultB>
): Parser<TResultB> =>
  map(([_, resultB]: [TResultA, TResultB]) => resultB)(pair(parserA, parserB));

const whitespace = oneOrMany(literal(" "));

const transformType: Parser<TransformType> = literal("s");

const transformInstruction: Parser<TransformInstruction> = map(
  (result: [TransformType, [string, string]]) => ({
    type: result[0],
    args: result[1]
  })
)(
  pair(
    transformType,
    right(
      literal("/"),
      pair(
        join(zeroOrMany(charBlacklist([" ", "/"]))),
        right(
          literal("/"),
          left(join(zeroOrMany(charBlacklist([" ", "/"]))), literal("/"))
        )
      )
    )
  )
);

const transformInstructionList: Parser<TransformInstruction[]> = map(
  (result: [TransformInstruction, TransformInstruction[]]) => [
    result[0],
    ...result[1]
  ]
)(
  pair(
    transformInstruction,
    zeroOrMany(right(whitespace, transformInstruction))
  )
);

export const parse = (input: Input) => {
  let result: TransformInstruction[];
  [result, input] = transformInstructionList(input);
  if (input.length !== 0) {
    throw new Error(`could not parse end of input ${input}`);
  }

  return result;
};
