import {
  Box,
  Card,
  CardProps,
  Image,
  Radio,
  Text,
  VStack,
  Wrap,
  WrapItem,
  useColorModeValue,
  useTheme,
} from "@chakra-ui/react";
import React, { useCallback, useLayoutEffect, useRef, useState } from "react";
import { Section, SectionProps } from "@/components/common/section";
import { useLauncherConfig } from "@/contexts/config";
import { omitFields } from "@/utils/object";
import cardStyles from "@/styles/card.module.css";

type WrapCardContentObject = {
  title: string | React.ReactNode;
  description: string;
  image?: string | React.ReactNode;
  extraContent?: React.ReactNode;
};

export interface WrapCardProps extends CardProps {
  cardContent?: React.ReactNode | WrapCardContentObject;
  colSpan?: number;
  variant?: "normal" | "radio";
  radioValue?: string;
  isSelected?: boolean;
  isFullClickZone?: boolean;
  onSelect?: () => void;
}

export interface WrapCardGroupProps extends SectionProps {
  cardMinWidth?: number;
  spacing?: number;
  items: WrapCardProps[];
  variant?: "normal" | "radio";
  widthMode?: "fixed" | "dynamic";
  cardAspectRatio?: number; // if not 0, calc height from width and aspect ratio
}

export const WrapCard: React.FC<WrapCardProps> = (props) => {
  const {
    cardContent,
    variant = "normal",
    radioValue = "",
    isSelected = false,
    isFullClickZone = false,
    onSelect,
    ...cardProps
  } = omitFields(props, ["colSpan"]);
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  const borderWidth = "1px";
  const hasBorder = isSelected || isFullClickZone;
  const theme = useTheme<{
    space: Record<string, string | number | undefined>;
  }>();
  const basePadding = cardProps.padding ?? 3;
  const subtractBorder = (value: string | number | null | undefined) => {
    if (value == null) return value;
    const token = theme.space[value] ?? value;
    const cssValue = typeof token === "number" ? `${token}px` : token;
    return `calc(${cssValue} - ${borderWidth})`;
  };
  const borderedPadding = Array.isArray(basePadding)
    ? basePadding.map((value) => subtractBorder(value) ?? null)
    : typeof basePadding === "object"
      ? Object.fromEntries(
          Object.entries(basePadding).map(
            ([key, value]: [string, string | number | undefined]) => [
              key,
              subtractBorder(value) ?? undefined,
            ]
          )
        )
      : (subtractBorder(basePadding) ?? undefined);
  const hoverBorderColor = useColorModeValue(
    `${primaryColor}.300`,
    `${primaryColor}.700`
  );

  const renderContent = () => {
    if (!cardContent) return null;

    if (React.isValidElement(cardContent) || typeof cardContent === "string") {
      return <>{cardContent}</>;
    }

    const { title, description, image, extraContent } =
      cardContent as WrapCardContentObject;

    return (
      <VStack spacing={0}>
        {image &&
          (typeof image === "string" ? (
            <Image
              boxSize="36px"
              objectFit="cover"
              src={image}
              alt={typeof title === "string" ? title : description}
            />
          ) : (
            image
          ))}
        {typeof title === "string" ? (
          <Text
            fontSize="xs-sm"
            className="ellipsis-text"
            fontWeight={isSelected ? "bold" : "normal"}
            mt={image ? 2 : 0}
            overflow="hidden"
          >
            {title}
          </Text>
        ) : (
          <Box mt={image ? 2 : 0} w="100%">
            {title}
          </Box>
        )}
        <Text fontSize="xs" className="secondary-text ellipsis-text">
          {description}
        </Text>
        {extraContent}
      </VStack>
    );
  };

  return (
    <Card
      className={cardStyles["card-front"]}
      borderColor={isSelected ? `${primaryColor}.500` : "transparent"}
      variant={isSelected ? "outline" : "elevated"}
      position="relative"
      borderWidth={hasBorder ? borderWidth : 0}
      p={hasBorder ? borderedPadding : basePadding}
      transition="border-color 0.15s ease-in-out, box-shadow 0.15s ease-in-out"
      {...(isFullClickZone && {
        cursor: "pointer",
        role: "button",
        tabIndex: 0,
        "aria-pressed": isSelected,
        _focusVisible: { boxShadow: "outline" },
        onKeyDown: (event: React.KeyboardEvent) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            onSelect?.();
          }
        },
        _hover: {
          borderColor: isSelected ? `${primaryColor}.500` : hoverBorderColor,
          boxShadow: "md",
        },
        _active: { boxShadow: "sm" },
        onClick: onSelect,
      })}
      {...cardProps}
    >
      {variant === "radio" && (
        <Box
          position="absolute"
          top={2}
          left={2}
          pointerEvents={isFullClickZone ? "none" : undefined}
        >
          <Radio
            value={radioValue}
            tabIndex={isFullClickZone ? -1 : undefined}
            onClick={isFullClickZone ? undefined : onSelect}
            colorScheme={primaryColor}
          />
        </Box>
      )}
      {renderContent()}
    </Card>
  );
};

export const WrapCardGroup: React.FC<WrapCardGroupProps> = ({
  cardMinWidth = 41.8, // in chakra-size, 4x rem (or px/4)
  spacing = 3.5, // in chakra-size
  items,
  variant = "normal",
  widthMode = "dynamic",
  cardAspectRatio = 0,
  ...props
}) => {
  const boxRef = useRef<HTMLDivElement>(null);
  const theme = useTheme<{
    space: Record<string, string | number | undefined>;
    fontSizes: Record<string, string>;
  }>();
  const spacingScale = theme.space[1] || "0.25rem";
  const baseFontSize = theme.fontSizes.base || "16px";
  const numberToPx = parseFloat(String(spacingScale)) * parseInt(baseFontSize);
  const [cardWidth, setCardWidth] = useState<number>(cardMinWidth * numberToPx); // width for single-column card (colSpan = 1)

  const resizeCard = useCallback(() => {
    if (boxRef.current && widthMode === "dynamic") {
      const boxWidth = boxRef.current.offsetWidth;
      const cardPerRow = Math.floor(
        (boxWidth + spacing * numberToPx) /
          (cardMinWidth * numberToPx + spacing * numberToPx)
      );
      if (items.length > cardPerRow) {
        const calculatedWidth =
          (boxWidth - spacing * numberToPx * (cardPerRow - 1) - 1) / cardPerRow;
        setCardWidth(calculatedWidth);
      } else {
        setCardWidth(cardMinWidth * numberToPx);
      }
    }
  }, [boxRef, numberToPx, cardMinWidth, spacing, widthMode, items.length]);

  useLayoutEffect(() => {
    if (!boxRef.current) return;
    const observer = new ResizeObserver(() => {
      resizeCard();
    });
    observer.observe(boxRef.current);

    return () => observer.disconnect();
  }, [resizeCard]);

  return (
    <Box {...props} ref={boxRef}>
      <Section
        title={props.title}
        headExtra={props.headExtra}
        description={props.description}
        isAccordion={props.isAccordion}
        initialIsOpen={props.initialIsOpen}
      >
        {items.length > 0 && (
          <Wrap spacing={spacing} mb={0.5}>
            {/* add mb to show last row cards' bottom shadow */}
            {items.map((item, index) => {
              const validColSpan = Math.max(1, Math.floor(item.colSpan || 1));
              const spannedWidth = `${cardWidth * validColSpan + (validColSpan - 1) * (spacing * numberToPx)}px`;
              return (
                <WrapItem key={index}>
                  <WrapCard
                    {...item}
                    overflow="hidden"
                    width={spannedWidth}
                    variant={variant}
                    {...(cardAspectRatio !== 0 && {
                      height: `${cardWidth / cardAspectRatio}px`,
                    })}
                  />
                </WrapItem>
              );
            })}
          </Wrap>
        )}
      </Section>
    </Box>
  );
};
