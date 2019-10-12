const transformBuildName = name => {
  const transform = process.env.BUILD_NAME_TRANSFORM;
  if (transform) {
    const [type, ...args] = transform.split("/");
    if (type === "s" && args.length === 3 && args[2] === "") {
      return name.replace(args[0], args[1]);
    } else {
      throw new Error(
        `Unknown transform ${transform}. Format has to be s/SEARCH/REPLACE/`
      );
    }
  }

  return name;
};

module.exports = {
  transformBuildName
};
