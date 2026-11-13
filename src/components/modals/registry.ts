import type { ComponentProps } from "react";
import AddAuthServerModal from "@/components/modals/add-auth-server-modal";
import AlertResourceDependencyModal from "@/components/modals/alert-resource-dependency-modal";
import CopyOrMoveModal from "@/components/modals/copy-or-move-modal";
import DeleteInstanceDialog from "@/components/modals/delete-instance-alert-dialog";
import DownloadModpackModal from "@/components/modals/download-modpack-modal";
import DownloadResourceModal from "@/components/modals/download-resource-modal";
import DownloadSpecificResourceModal from "@/components/modals/download-specific-resource-modal";
import ExtensionInfoModal from "@/components/modals/extension-info-modal";
import GenericConfirmDialog from "@/components/modals/generic-confirm-dialog";
import ImportModpackModal from "@/components/modals/import-modpack-modal";
import LaunchProcessModal from "@/components/modals/launch-process-modal";
import NotifyNewVersionModal from "@/components/modals/notify-new-version-modal";
import ReLoginPlayerModal from "@/components/modals/relogin-player-modal";
import SpotlightSearchModal from "@/components/modals/spotlight-search-modal";

export const sharedModals = {
  "add-auth-server": AddAuthServerModal,
  "alert-resource-dependency": AlertResourceDependencyModal,
  "copy-or-move": CopyOrMoveModal,
  "delete-instance-alert": DeleteInstanceDialog,
  "download-modpack": DownloadModpackModal,
  "download-resource": DownloadResourceModal,
  "download-specific-resource": DownloadSpecificResourceModal,
  "extension-info": ExtensionInfoModal,
  "generic-confirm": GenericConfirmDialog,
  "import-modpack": ImportModpackModal,
  launch: LaunchProcessModal,
  "notify-new-version": NotifyNewVersionModal,
  relogin: ReLoginPlayerModal,
  "spotlight-search": SpotlightSearchModal,
};

export type SharedModalKey = keyof typeof sharedModals;
export type SharedModalParams<Key extends SharedModalKey> = Omit<
  ComponentProps<(typeof sharedModals)[Key]>,
  "isOpen" | "onClose"
> & { onCloseCallback?: () => void };
export type SharedModalArguments = {
  [Key in SharedModalKey]: {} extends SharedModalParams<Key>
    ? [key: Key, params?: SharedModalParams<Key>, options?: { stack?: boolean }]
    : [key: Key, params: SharedModalParams<Key>, options?: { stack?: boolean }];
}[SharedModalKey];
export type OpenSharedModal = (...args: SharedModalArguments) => void;
