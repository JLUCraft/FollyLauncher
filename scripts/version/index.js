const path = require("path");

const subCommand = process.argv[2];

if (!subCommand) {
  console.error("Please specify a subcommand for version (check or bump)");
  process.exit(1);
}

try {
  require(path.join(__dirname, `${subCommand}.js`));
} catch (error) {
  if (
    error instanceof Error &&
    "code" in error &&
    error.code === "MODULE_NOT_FOUND"
  ) {
    console.error(`Unknown subcommand: ${subCommand}`);
    console.error("Available subcommands: check, bump");
  } else {
    console.error(error);
  }
  process.exit(1);
}
