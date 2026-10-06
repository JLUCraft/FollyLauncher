export const isDev = process.env.NODE_ENV === "development";

export const isProd = process.env.NODE_ENV === "production";

export const parseModernWindowsVersion = (version: string): string => {
  // Windows 10: version starts with 10.0 and build < 21996
  // Windows 11: version starts with 10.0 and build >= 21996
  const match = version.match(/^10\.0\.(\d+)/);
  if (match) {
    const build = parseInt(match[1], 10);
    return `${build >= 21996 ? "11" : "10"} (${version})`;
  }
  return version;
};
