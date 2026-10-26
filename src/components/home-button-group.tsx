import {
  Box,
  Button,
  Flex,
  HStack,
  IconButton,
  IconButtonProps,
  Popover,
  PopoverBody,
  PopoverContent,
  PopoverTrigger,
  Text,
  Tooltip,
  VStack,
  useDisclosure,
  useToken,
} from "@chakra-ui/react";
import { useRouter } from "next/router";
import { cloneElement, useState } from "react";
import { useTranslation } from "react-i18next";
import { LuArrowLeftRight, LuPlus, LuSettings } from "react-icons/lu";
import AdvancedCard from "@/components/common/advanced-card";
import { CommonIconButton } from "@/components/common/common-icon-button";
import InstancesView from "@/components/instances-view";
import PlayerAvatar from "@/components/player-avatar";
import PlayersView from "@/components/players-view";
import { useLauncherConfig } from "@/contexts/config";
import { useGlobalData } from "@/contexts/global-data";
import { useSharedModals } from "@/contexts/shared-modal";
import { PlayerType } from "@/enums/account";
import styles from "@/styles/launch.module.css";
import {
  LAUNCH_ACCOUNT_SETUP_PATH,
  LAUNCH_INSTANCE_SETUP_PATH,
} from "@/utils/launch-flow";

interface CustomButtonProps extends Omit<
  IconButtonProps,
  "onClick" | "aria-label"
> {
  tooltip: string;
  onClick: () => void;
  popoverContent: React.ReactElement;
  showAdd?: boolean;
  onAddClick?: () => void;
}

const ButtonWithPopover: React.FC<CustomButtonProps> = ({
  tooltip,
  popoverContent,
  onClick,
  showAdd = false,
  onAddClick,
  ...props
}) => {
  const { config } = useLauncherConfig();
  const quickSwitch = config.general.functionality.launchPageQuickSwitch;
  const { isOpen, onToggle, onClose } = useDisclosure();
  const [tooltipDisabled, setTooltipDisabled] = useState(false);

  const handleClose = () => {
    setTooltipDisabled(true);
    onClose();
    setTimeout(() => setTooltipDisabled(false), 200);
  };

  return (
    <Popover
      isOpen={showAdd ? false : isOpen}
      onClose={handleClose}
      placement="top-end"
      gutter={12}
    >
      <Tooltip label={tooltip} placement="top-end" isDisabled={tooltipDisabled}>
        <Box lineHeight={0}>
          <PopoverTrigger>
            <IconButton
              size="xs"
              icon={showAdd ? <LuPlus /> : <LuArrowLeftRight />}
              {...props}
              aria-label={tooltip}
              onClick={() => {
                if (showAdd) return (onAddClick ?? onClick)();
                quickSwitch ? onToggle() : onClick();
              }}
            />
          </PopoverTrigger>
        </Box>
      </Tooltip>

      {!showAdd && (
        <PopoverContent maxH="3xs" overflow="auto">
          <PopoverBody p={0}>
            {cloneElement(popoverContent, {
              onSelectCallback: () => setTimeout(handleClose, 100),
            })}
          </PopoverBody>
        </PopoverContent>
      )}
    </Popover>
  );
};

const HomeButtonGroup = () => {
  const { t } = useTranslation();
  const router = useRouter();
  const { openSharedModal } = useSharedModals();
  const { config } = useLauncherConfig();
  const { selectedPlayer, selectedInstance, getPlayerList, getInstanceList } =
    useGlobalData();

  const primaryColor = config.appearance.theme.primaryColor;
  const [glowBase, glowHover, glowActive] = useToken("colors", [
    `${primaryColor}.400`,
    `${primaryColor}.500`,
    `${primaryColor}.600`,
  ]);

  // glow shadows for the launch button, following the theme primary color
  const toRgba = (hex: string, alpha: number) => {
    const n = parseInt(hex.slice(1), 16);
    return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`;
  };
  const glowShadow = (color: string, alpha: number, spread: number) =>
    [
      `-${spread}px 0 5px ${spread}px`,
      `0px 0 5px ${spread}px`,
      `${spread}px 0 5px ${spread}px`,
    ]
      .map((shadow) => `${shadow} ${toRgba(color, alpha)}`)
      .join(", ");

  const players = getPlayerList();
  const instances = getInstanceList();
  const playerList = players || [];
  const instanceList = instances || [];
  const hasPlayers = playerList.length > 0;
  const hasInstances = instanceList.length > 0;
  const launchLabel = !hasInstances
    ? t("AllInstancesPage.button.addAndImport")
    : !selectedInstance
      ? t("SelectInstanceModal.header.title")
      : !hasPlayers
        ? t("AccountsPage.button.addPlayer")
        : !selectedPlayer
          ? t("SelectPlayerModal.header.title")
          : t("LaunchPage.button.launch");

  return (
    <Flex wrap="wrap" justify="flex-end" align="flex-end" gap={4}>
      <AdvancedCard className={styles["selected-user-card"]}>
        <Box position="absolute" top={1} right={1}>
          <ButtonWithPopover
            tooltip={t(
              `LaunchPage.SwitchButton.tooltip.${hasPlayers ? "switchPlayer" : "addPlayer"}`
            )}
            variant="subtle"
            isDisabled={players === undefined}
            popoverContent={
              <PlayersView
                players={playerList}
                selectedPlayer={selectedPlayer}
                viewType="list"
                withMenu={false}
              />
            }
            onClick={() => router.push("/accounts")}
            showAdd={!hasPlayers}
            onAddClick={() => router.push(LAUNCH_ACCOUNT_SETUP_PATH)}
          />
        </Box>

        <HStack spacing={2.5} h="100%" w="100%">
          {selectedPlayer ? (
            <>
              {/* Only reserve avatar space when a skin face is available;
                  an empty avatar box would leave the left half of the card blank. */}
              {!!selectedPlayer.avatar?.[0] && (
                <PlayerAvatar
                  boxSize="32px"
                  objectFit="cover"
                  avatar={selectedPlayer.avatar}
                />
              )}
              <VStack spacing={0} align="left" mt={-2} minW={0} pr={6}>
                <Text
                  fontSize="xs-sm"
                  fontWeight="bold"
                  maxW="100%"
                  mt={2}
                  isTruncated
                >
                  {selectedPlayer.name}
                </Text>
                <Text fontSize="2xs" className="secondary-text">
                  {t(
                    `Enums.playerTypes.${selectedPlayer.playerType === PlayerType.ThirdParty ? "3rdpartyShort" : selectedPlayer.playerType}`
                  )}
                </Text>
                <Text fontSize="2xs" className="secondary-text">
                  {selectedPlayer.playerType === PlayerType.ThirdParty &&
                    selectedPlayer.authServer?.name}
                </Text>
              </VStack>
            </>
          ) : (
            <Button
              variant="ghost"
              w="100%"
              h="100%"
              whiteSpace="normal"
              isDisabled={players === undefined}
              onClick={() =>
                void router.push(
                  hasPlayers ? "/accounts" : LAUNCH_ACCOUNT_SETUP_PATH
                )
              }
            >
              {t(
                hasPlayers
                  ? "SelectPlayerModal.header.title"
                  : "AccountsPage.button.addPlayer"
              )}
            </Button>
          )}
        </HStack>
      </AdvancedCard>

      <HStack align="flex-end" spacing={2}>
        {/* Auxiliary actions stand beside the launch button as normal buttons,
            so they no longer sit inside the main launch hit area. */}
        <VStack spacing={1.5} pb={1}>
          {selectedInstance && hasInstances && (
            <CommonIconButton
              icon={LuSettings}
              label={t("LaunchPage.button.instanceSettings")}
              tooltipPlacement="left"
              onClick={() =>
                router.push({
                  pathname: "/instances/details/[id]/settings",
                  query: { id: selectedInstance.id },
                })
              }
            />
          )}

          <ButtonWithPopover
            isDisabled={instances === undefined}
            tooltip={t(
              `LaunchPage.SwitchButton.tooltip.${hasInstances ? "switchInstance" : "addInstance"}`
            )}
            popoverContent={
              <InstancesView
                instances={instanceList}
                selectedInstance={selectedInstance}
                viewType="list"
                withMenu={false}
              />
            }
            onClick={() => router.push("/instances/list")}
            showAdd={!hasInstances}
            onAddClick={() => router.push(LAUNCH_INSTANCE_SETUP_PATH)}
          />
        </VStack>

        <Button
          id="main-launch-button"
          colorScheme="blackAlpha"
          className={styles["launch-button"]}
          isLoading={players === undefined || instances === undefined}
          boxShadow={glowShadow(glowBase, 0.3, 1)}
          _hover={{ boxShadow: glowShadow(glowHover, 0.5, 2) }}
          _active={{ boxShadow: glowShadow(glowActive, 0.7, 3) }}
          onClick={() => {
            if (!hasInstances)
              return void router.push(LAUNCH_INSTANCE_SETUP_PATH);
            if (selectedInstance && !hasPlayers)
              return void router.push(LAUNCH_ACCOUNT_SETUP_PATH);
            openSharedModal("launch", {
              instanceId: selectedInstance?.id ?? "tbd",
              ...(!selectedPlayer && { playerId: "tbd" }),
            });
          }}
        >
          <VStack spacing={1.5} w="100%" color="white">
            <Text fontSize="lg" fontWeight="bold">
              {launchLabel}
            </Text>
            <Text fontSize="sm" className="ellipsis-text">
              {selectedInstance
                ? selectedInstance.name
                : t("LaunchPage.Text.noSelectedGame")}
            </Text>
          </VStack>
        </Button>
      </HStack>
    </Flex>
  );
};

export default HomeButtonGroup;
