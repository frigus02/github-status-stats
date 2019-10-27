import { optionalEnv } from "./env";
import { parse, TransformInstruction } from "./transform-parser";

const identityTransform = (x: string) => x;

const createTransform = (transformString: string) => {
  let transforms: TransformInstruction[];
  try {
    transforms = parse(transformString);
  } catch (e) {
    throw new Error(
      `Error parsing transform ${transformString}: ${e.message}. Format has to be s/SEARCH/REPLACE/. Space and slash have to be escaped. Multiple transforms are space separated.`
    );
  }

  return (x: string) =>
    transforms.reduce((x, transform) => {
      switch (transform.type) {
        case "s":
          return x.replace(transform.args[0], transform.args[1]);
      }
    }, x);
};

export const transformStatusContext = optionalEnv.STATUS_CONTEXT_TRANSFORM
  ? createTransform(optionalEnv.STATUS_CONTEXT_TRANSFORM)
  : identityTransform;
