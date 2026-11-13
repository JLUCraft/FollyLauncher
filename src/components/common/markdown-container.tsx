import {
  Box,
  BoxProps,
  Divider,
  Heading,
  Image,
  Link,
  ListItem,
  OrderedList,
  Table,
  TableContainer,
  Tbody,
  Td,
  Text,
  Th,
  Thead,
  Tr,
  UnorderedList,
} from "@chakra-ui/react";
import { openUrl } from "@tauri-apps/plugin-opener";
import React from "react";
import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { useLauncherConfig } from "@/contexts/config";
import { omitFields } from "@/utils/object";

type MarkdownContainerProps = BoxProps & {
  children: string;
};

const MarkdownContainer: React.FC<MarkdownContainerProps> = ({
  children,
  ...boxProps
}) => {
  const { config } = useLauncherConfig();
  const primaryColor = config.appearance.theme.primaryColor;

  // process GitHub-style mentions and issue / PR references
  const processGitHubMarks = (children: React.ReactNode): React.ReactNode => {
    if (typeof children === "string") {
      const parts = children.split(/(#[0-9]+|@[a-zA-Z0-9_-]+)/g);
      return parts.map((part, idx) => {
        if (/^#[0-9]+$/.test(part)) {
          const issueNumber = part.substring(1);
          return (
            <Link
              key={idx}
              color={`${primaryColor}.500`}
              onClick={() =>
                openUrl(
                  `https://github.com/UNIkeEN/SJMCL/pull/${issueNumber}`
                ).catch(logger.error)
              }
            >
              {part}
            </Link>
          );
        }
        if (/^@[a-zA-Z0-9_-]+$/.test(part)) {
          const username = part.substring(1);
          return (
            <Link
              key={idx}
              color={`${primaryColor}.500`}
              onClick={() =>
                openUrl(`https://github.com/${username}`).catch(logger.error)
              }
            >
              {part}
            </Link>
          );
        }
        return <React.Fragment key={idx}>{part}</React.Fragment>;
      });
    }

    if (Array.isArray(children)) {
      return children.map((child: React.ReactNode, i) => (
        <React.Fragment key={i}>{processGitHubMarks(child)}</React.Fragment>
      ));
    }

    if (React.isValidElement<{ children?: React.ReactNode }>(children)) {
      const childProps = children.props?.children ?? null;
      return React.cloneElement(children, {
        ...children.props,
        children: processGitHubMarks(childProps),
      });
    }

    return children;
  };

  // map HTML tags to Chakra components so styles are inherited.
  const components: Components = {
    // paragraphs
    p: ({ children, ...rest }) => (
      <Text {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </Text>
    ),
    // headings
    h1: ({ children, ...rest }) => (
      <Heading as="h1" size="xl" my={4} {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </Heading>
    ),
    h2: ({ children, ...rest }) => (
      <Heading as="h2" size="lg" my={3} {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </Heading>
    ),
    h3: ({ children, ...rest }) => (
      <Heading as="h3" size="md" my={2} {...omitFields(rest, ["node"])}>
        {children}
      </Heading>
    ),
    h4: ({ children, ...rest }) => (
      <Heading as="h4" size="sm" my={2} {...omitFields(rest, ["node"])}>
        {children}
      </Heading>
    ),
    strong: ({ children, ...rest }) => (
      <Text
        as="strong"
        fontWeight="600"
        color="inherit"
        {...omitFields(rest, ["node"])}
      >
        {processGitHubMarks(children)}
      </Text>
    ),
    em: ({ children, ...rest }) => (
      <Text
        as="em"
        fontStyle="italic"
        color="inherit"
        {...omitFields(rest, ["node"])}
      >
        {processGitHubMarks(children)}
      </Text>
    ),
    // divider
    hr: ({ ...rest }) => <Divider my={4} {...omitFields(rest, ["node"])} />,
    // links
    a: ({ href, children, ...rest }) => (
      <Link
        _hover={{ textDecoration: "underline" }}
        color={`${primaryColor}.500`}
        onClick={(e) => {
          e.preventDefault();
          if (href)
            openUrl(href).catch((error: unknown) => console.error(error));
        }}
        {...omitFields(rest, ["node"])}
      >
        {children}
      </Link>
    ),
    // lists
    ul: ({ children, ...rest }) => (
      <UnorderedList pl={5} my={2} {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </UnorderedList>
    ),
    ol: ({ children, ...rest }) => (
      <OrderedList pl={5} my={2} {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </OrderedList>
    ),
    li: ({ children, ...rest }) => (
      <ListItem my={1} {...omitFields(rest, ["node"])}>
        {processGitHubMarks(children)}
      </ListItem>
    ),
    // images
    img: ({ src, alt, ...rest }) => (
      <Image
        src={typeof src === "string" ? src : undefined}
        alt={alt}
        maxW="100%"
        my={2}
        borderRadius="md"
        {...omitFields(rest, ["node"])}
      />
    ),
    // tables
    table: ({ children, ...rest }) => (
      <TableContainer>
        <Table variant="simple" size="sm" {...omitFields(rest, ["node"])}>
          {children}
        </Table>
      </TableContainer>
    ),
    thead: ({ children, ...rest }) => (
      <Thead {...omitFields(rest, ["node"])}>{children}</Thead>
    ),
    tbody: ({ children, ...rest }) => (
      <Tbody {...omitFields(rest, ["node"])}>{children}</Tbody>
    ),
    tr: ({ children, ...rest }) => (
      <Tr {...omitFields(rest, ["node"])}>{children}</Tr>
    ),
    th: ({ children, ...rest }) => (
      <Th {...omitFields(rest, ["node"])}>{children}</Th>
    ),
    td: ({ children, ...rest }) => (
      <Td {...omitFields(rest, ["node"])}>{children}</Td>
    ),
  };

  return (
    <Box {...boxProps}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
        {children || ""}
      </ReactMarkdown>
    </Box>
  );
};

export default MarkdownContainer;
