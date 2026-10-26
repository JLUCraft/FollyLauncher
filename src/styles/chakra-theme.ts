import { extendTheme } from "@chakra-ui/react";
import { mode } from "@chakra-ui/theme-tools";
import { brandColors, brandSemantic } from "@/styles/brand-theme.gen";

const subtleButtonVariant = (props: any) => ({
  bg: mode(`${props.colorScheme}.100`, `${props.colorScheme}.700`)(props),
  color: mode(`${props.colorScheme}.700`, `${props.colorScheme}.100`)(props),
  _hover: {
    bg: mode(`${props.colorScheme}.200`, `${props.colorScheme}.600`)(props),
  },
  _active: {
    bg: mode(`${props.colorScheme}.300`, `${props.colorScheme}.500`)(props),
  },
  _disabled: {
    bg: mode(`${props.colorScheme}.100`, `${props.colorScheme}.700`)(props),
    color: mode(`${props.colorScheme}.500`, `${props.colorScheme}.300`)(props),
    cursor: "not-allowed",
    opacity: 0.6,
    _hover: {
      bg: mode(
        `${props.colorScheme}.100 !important`,
        `${props.colorScheme}.700 !important`
      )(props),
    },
    _active: {
      bg: mode(
        `${props.colorScheme}.100 !important`,
        `${props.colorScheme}.700 !important`
      )(props),
    },
  },
});

const customGraySolidButtonTheme = {
  bg: "gray.600",
  color: "white",
  _hover: { bg: "gray.700" },
  _active: { bg: "gray.800" },
  _disabled: {
    bg: "gray.500",
    color: "gray.300",
    cursor: "not-allowed",
    opacity: 0.6,
    _hover: {
      bg: "gray.500 !important",
    },
    _active: {
      bg: "gray.500 !important",
    },
  },
};

/**
 * 组件默认色：Chakra 自带默认是 blue，这里统一改到校徽取色得到的 brand 色阶，
 * 这样不改上游组件也能让整个界面用上本社团的品牌色。语义色（red 危险、green
 * 成功、orange 提醒）保持原样——它们表达状态，不该随学校变色。
 */
const brandDefault = { defaultProps: { colorScheme: "brand" } };

/** 品牌语义令牌 → Chakra semanticTokens，按深浅模式自动切换。 */
const brandSemanticTokens = Object.fromEntries(
  Object.entries(brandSemantic.light).map(([name, value]) => [
    `brand.${name}`,
    {
      default: value,
      _dark: brandSemantic.dark[name as keyof typeof brandSemantic.dark],
    },
  ])
);

const chakraExtendTheme = extendTheme({
  initialColorMode: "dark",
  useSystemColorMode: false,
  components: {
    Alert: {
      defaultProps: { colorScheme: "brand" },
      baseStyle: {
        container: {
          paddingY: 2,
        },
        title: {
          fontSize: "sm",
        },
        description: {
          fontSize: "xs-sm",
          lineHeight: 4,
        },
        icon: {
          alignSelf: "center",
        },
      },
    },
    Badge: brandDefault,
    Button: {
      baseStyle: {
        fontWeight: "normal",
        _focusVisible: { boxShadow: "outline" },
      },
      defaultProps: {
        size: "sm",
      },
      variants: {
        subtle: subtleButtonVariant,
        solid: (props: any) => {
          return props.colorScheme === "gray" ? customGraySolidButtonTheme : {};
        },
      },
    },
    Checkbox: brandDefault,
    Divider: {
      baseStyle: {
        borderColor: "var(--chakra-colors-chakra-border-color)", // prevent overwrite
      },
    },
    FormLabel: {
      baseStyle: {
        fontSize: "sm",
      },
    },
    Input: {
      defaultProps: {
        size: "sm",
        focusBorderColor: "brand.400",
      },
    },
    Link: {
      baseStyle: {
        color: "brand.600",
        _dark: { color: "brand.300" },
      },
    },
    Menu: {
      baseStyle: {
        groupTitle: {
          fontSize: "xs",
        },
        list: {
          minWidth: "auto",
          py: 1,
        },
      },
    },
    Modal: {
      // Modal 本身不取色，这里只保留原有的尺寸微调。
      baseStyle: {
        dialog: {
          header: {
            fontSize: "md",
            paddingLeft: 4,
            paddingRight: 8, // reserve space for close button (issue#1312)
            paddingTop: 3,
            paddingBottom: 2,
            userSelect: "none",
            cursor: "default",
          },
          footer: {
            fontSize: "sm",
            gap: 3,
            marginTop: "auto",
            paddingX: 4,
            paddingTop: 2,
            paddingBottom: 4,
            userSelect: "none",
          },
        },
        closeButton: {
          top: 2,
          right: 2,
          width: 6,
          height: 6,
          fontSize: "2xs",
        },
        body: {
          fontSize: "sm",
          paddingX: 4,
          userSelect: "none",
        },
      },
      defaultProps: {
        isCentered: true,
      },
    },
    Progress: brandDefault,
    Radio: brandDefault,
    Popover: {
      baseStyle: {
        content: {
          boxShadow: "md",
        },
      },
    },
    Slider: brandDefault,
    Spinner: brandDefault,
    Switch: {
      defaultProps: {
        size: "sm",
        colorScheme: "brand",
      },
    },
    Tabs: brandDefault,
    Tag: {
      defaultProps: {
        size: "sm",
        colorScheme: "brand",
        whiteSpace: "nowrap",
      },
    },
    Tooltip: {
      baseStyle: {
        userSelect: "none",
        fontSize: "xs-sm",
      },
    },
  },
  colors: {
    // 校徽取色得到的色阶（生成文件，见 brand-palette 仓库）。新组件用
    // colorScheme="brand"，或在需要按模式取值时写 mode("brand.700", "brand.300")。
    brand: brandColors,
    overlay: "rgba(0, 0, 0, 0.45)",
    overlayStrong: "rgba(0, 0, 0, 0.6)",
  },
  fontSizes: {
    "xs-sm": "0.8rem",
  },
  shadows: {
    outline: "0 0 0 3px var(--chakra-colors-brand-400)",
  },
  semanticTokens: {
    colors: brandSemanticTokens,
  },
});

export default chakraExtendTheme;
