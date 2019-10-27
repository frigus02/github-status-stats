const keys = [
  "GH_TOKEN",
  "GH_OWNER",
  "GH_REPO",
  "GH_COMMITS_SINCE",
  "GH_COMMITS_UNTIL",
  "STATUS_CONTEXT_TRANSFORM",
  "BUILD_CANCELED_REGEX"
] as const;
type Keys = typeof keys;
type Key = Keys[number];

const env = {} as Record<Key, string>;
for (const key of keys) {
  Object.defineProperty(env, key, {
    get() {
      const value = process.env[key];
      if (!value) {
        throw new Error(`Environment variable ${key} is not set`);
      }

      return value;
    },
    enumerable: true
  });
}

const optionalEnv = process.env as Record<Key, string | undefined>;

export { env, optionalEnv };
