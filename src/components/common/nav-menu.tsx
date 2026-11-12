import { Stack, StackProps, Tooltip } from "@chakra-ui/react";
import React from "react";
import SelectableButton from "@/components/common/selectable-button";

export interface MenuItem<Value extends string = string> {
  label: React.ReactNode;
  value: Value;
  tooltip?: string;
}

export interface NavMenuProps<Value extends string = string> extends Omit<
  StackProps,
  "onClick"
> {
  items: MenuItem<Value>[];
  selectedKeys?: Value[];
  onClick?: (value: Value) => void;
  size?: string;
}

const NavMenu = <Value extends string>({
  items,
  selectedKeys = [],
  onClick,
  size = "sm",
  ...props
}: NavMenuProps<Value>) => {
  return (
    <Stack align="stretch" spacing={0.5} {...props}>
      {items.map((item) => (
        <Tooltip label={item.tooltip} placement="right" key={item.value}>
          <Stack align="stretch" direction="column">
            <SelectableButton
              key={item.value}
              size={size}
              isSelected={selectedKeys.includes(item.value)}
              onClick={() => onClick && onClick(item.value)}
            >
              {item.label}
            </SelectableButton>
          </Stack>
        </Tooltip>
      ))}
    </Stack>
  );
};

export default NavMenu;
