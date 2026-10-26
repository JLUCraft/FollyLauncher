/* 生成文件，请勿手改。
 * 来源：brands/jlu/brand.json
 * 生成器：brand-palette --chakra
 */

/** 徽标取色得到的完整色阶。用于 `colors.brand`，例如 `brand.600`。 */
export const brandColors = {
  50: "#f2f7ff",
  100: "#e3edff",
  200: "#caddff",
  300: "#a4c5ff",
  400: "#75a4f7",
  500: "#5183df",
  600: "#3768c1",
  700: "#234e9d",
  800: "#153775",
  900: "#0c2553",
  950: "#031336",
} as const;

/** 语义令牌，light/dark 两套。需要按模式取值时用 Chakra 的 mode() 取对应项。 */
export const brandSemantic = {
  light: {
    bg: "#f5f7f9",
    surface: "#ffffff",
    "surface-alt": "#ebedf0",
    line: "#dadce1",
    "line-strong": "#82868e",
    ink: "#26282b",
    muted: "#4f5359",
    primary: "#234e9d",
    "primary-hover": "#153775",
    "primary-text": "#234e9d",
    "primary-subtle": "#f2f7ff",
    "primary-border": "#caddff",
    "on-primary": "#ffffff",
    secondary: "#5e6997",
    "on-secondary": "#ffffff",
    accent: "#3768c1",
    focus: "#3768c1",
    success: "#03642c",
    "on-success": "#ffffff",
    "success-bg": "#e6feea",
    error: "#932421",
    "on-error": "#ffffff",
    "error-bg": "#fff4f2",
  },
  dark: {
    bg: "#151619",
    surface: "#26282b",
    "surface-alt": "#383b3f",
    line: "#383b3f",
    "line-strong": "#82868e",
    ink: "#f4f7fc",
    muted: "#c0c4cb",
    primary: "#234e9d",
    "primary-hover": "#3768c1",
    "primary-text": "#a4c5ff",
    "primary-subtle": "#031336",
    "primary-border": "#153775",
    "on-primary": "#ffffff",
    secondary: "#97a2cf",
    "on-secondary": "#151619",
    accent: "#a4c5ff",
    focus: "#70a3ff",
    success: "#98d6a4",
    "on-success": "#151619",
    "success-bg": "#003111",
    error: "#ffaba1",
    "on-error": "#151619",
    "error-bg": "#4d0b0a",
  },
} as const;

export type BrandColorStep = keyof typeof brandColors;
