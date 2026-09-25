import { Badge, Box, Flex, HStack, Text, VStack } from "@chakra-ui/react";
import type { ReactNode } from "react";
import { useLauncherConfig } from "@/contexts/config";

export function Scoreboard<T extends number | string>({
  title,
  entries,
  emptyText,
  formatScore = (score) => score,
  rankVariant = "text",
  compact = false,
}: {
  title: string;
  entries: [string, T][];
  emptyText: string;
  formatScore?: (score: T) => ReactNode;
  rankVariant?: "text" | "badge";
  compact?: boolean;
}) {
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;
  return (
    <Box>
      <Text
        fontWeight={compact ? "semibold" : "bold"}
        fontSize="sm"
        mb={compact ? 1 : 2}
      >
        {title}
      </Text>
      {entries.length === 0 ? (
        <Text fontSize={compact ? "xs" : "sm"} className="secondary-text">
          {emptyText}
        </Text>
      ) : (
        <VStack spacing={1} align="stretch">
          {entries.map(([name, score], index) =>
            rankVariant === "badge" ? (
              <HStack key={name} fontSize="sm" spacing={2}>
                {index < 3 ? (
                  <Badge colorScheme={index === 0 ? primaryColor : "gray"}>
                    {index + 1}
                  </Badge>
                ) : (
                  <Text w="2em" className="secondary-text">
                    {index + 1}
                  </Text>
                )}
                <Text>{name}</Text>
                <Text ml="auto" className="secondary-text">
                  {formatScore(score)}
                </Text>
              </HStack>
            ) : (
              <Flex key={name} fontSize="sm" align="center">
                <Text
                  w={8}
                  flexShrink={0}
                  fontWeight={index < 3 ? "bold" : "normal"}
                  color={index < 3 ? `${primaryColor}.500` : "gray.500"}
                >
                  {index + 1}
                </Text>
                <Text flex="1" overflowWrap="anywhere">
                  {name}
                </Text>
                <Text fontWeight="medium" flexShrink={0}>
                  {formatScore(score)}
                </Text>
              </Flex>
            )
          )}
        </VStack>
      )}
    </Box>
  );
}
