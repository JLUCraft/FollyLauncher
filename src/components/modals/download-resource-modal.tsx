import {
  Flex,
  HStack,
  Icon,
  Modal,
  ModalBody,
  ModalCloseButton,
  ModalContent,
  ModalHeader,
  ModalOverlay,
  ModalProps,
  Text,
  Tooltip,
} from "@chakra-ui/react";
import { useRouter } from "next/router";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconType } from "react-icons";
import {
  LuEarth,
  LuHaze,
  LuPackage,
  LuPuzzle,
  LuSquareLibrary,
} from "react-icons/lu";
import NavMenu from "@/components/common/nav-menu";
import ResourceDownloader from "@/components/resource-downloader";
import { useLauncherConfig } from "@/contexts/config";
import { useGlobalData } from "@/contexts/global-data";
import { OtherResourceSource, OtherResourceType } from "@/enums/resource";

interface DownloadResourceModalProps extends Omit<ModalProps, "children"> {
  initialResourceType?: OtherResourceType;
  initialSearchQuery?: string;
  initialDownloadSource?: OtherResourceSource;
}

const DownloadResourceModal: React.FC<DownloadResourceModalProps> = ({
  initialResourceType = OtherResourceType.Mod,
  initialSearchQuery = "",
  initialDownloadSource = OtherResourceSource.CurseForge,
  ...modalProps
}) => {
  const { t } = useTranslation();
  const { isZh } = useLauncherConfig();
  const router = useRouter();
  const { getInstanceList, instanceList } = useGlobalData();

  const [selectedResourceType, setSelectedResourceType] =
    useState<OtherResourceType>(initialResourceType);
  const { id } = router.query;
  const instanceId = Array.isArray(id) ? id[0] : id;
  const curInstance = instanceList?.find(
    (instance) => instance.id === instanceId
  );

  const resourceTypeList: { key: OtherResourceType; icon: IconType }[] = [
    { key: OtherResourceType.Mod, icon: LuSquareLibrary },
    { key: OtherResourceType.World, icon: LuEarth },
    { key: OtherResourceType.ResourcePack, icon: LuPackage },
    { key: OtherResourceType.ShaderPack, icon: LuHaze },
    { key: OtherResourceType.DataPack, icon: LuPuzzle },
  ];

  useEffect(() => {
    getInstanceList();
  }, [getInstanceList]);

  return (
    <Modal
      scrollBehavior="inside"
      size={{ base: "2xl", lg: "3xl", xl: "4xl" }}
      returnFocusOnClose={false}
      {...modalProps}
    >
      <ModalOverlay />
      <ModalContent h="100%" pb={4}>
        <ModalHeader>
          <HStack w="100%" justify="flex-start" align="center">
            <Text>{t("DownloadResourceModal.header.title")}</Text>
            <NavMenu<OtherResourceType>
              className="no-scrollbar"
              overflowX="auto"
              selectedKeys={[selectedResourceType]}
              onClick={(value) => setSelectedResourceType(value)}
              direction="row"
              size="xs"
              spacing={3}
              mr={4}
              flex={1}
              display="flex"
              justify="center"
              items={resourceTypeList.map((item) => ({
                value: item.key,
                label: (
                  <Tooltip
                    label={t(
                      `DownloadResourceModal.resourceTypeList.${item.key}`
                    )}
                    isDisabled={isZh}
                  >
                    <HStack spacing={1.5} fontSize="sm">
                      <Icon as={item.icon} />
                      {(isZh || selectedResourceType === item.key) && (
                        <Text>
                          {t(
                            `DownloadResourceModal.resourceTypeList.${item.key}`
                          )}
                        </Text>
                      )}
                    </HStack>
                  </Tooltip>
                ),
              }))}
            />
          </HStack>
        </ModalHeader>
        <ModalCloseButton />
        <Flex flexGrow="1" flexDir="column">
          <ModalBody>
            <ResourceDownloader
              // key={selectedResourceType}
              resourceType={selectedResourceType}
              initialSearchQuery={initialSearchQuery}
              initialDownloadSource={initialDownloadSource}
              curInstance={curInstance}
            />
          </ModalBody>
        </Flex>
      </ModalContent>
    </Modal>
  );
};

export default DownloadResourceModal;
