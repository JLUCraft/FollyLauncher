import { BoxProps, HStack, Heading, Highlight, Image } from "@chakra-ui/react";
import { brand } from "@/brand";
import styles from "@/styles/logo-title.module.css";

interface LogoTitleProps extends BoxProps {}

export const TitleShort: React.FC<LogoTitleProps> = (props) => {
  return (
    <Heading size="md" className={styles.title} {...props}>
      <Highlight
        query={brand.shortName.slice(-1)}
        styles={{ color: "blue.600", userSelect: "none" }}
      >
        {brand.shortName}
      </Highlight>
    </Heading>
  );
};

export const TitleFull: React.FC<LogoTitleProps> = (props) => {
  return (
    <Heading size="md" className={styles.title} {...props}>
      <Highlight
        query={brand.shortName.slice(-1)}
        styles={{ color: "blue.600", userSelect: "none" }}
      >
        {brand.productName}
      </Highlight>
    </Heading>
  );
};

export const TitleFullWithLogo: React.FC<LogoTitleProps> = (props) => {
  return (
    <HStack>
      <Image src="/images/icons/Logo_128x128.png" alt="Logo" boxSize="36px" />
      <TitleFull {...props} />
    </HStack>
  );
};
