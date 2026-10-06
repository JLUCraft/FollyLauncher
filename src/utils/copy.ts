import { Image } from "@tauri-apps/api/image";
import { writeImage, writeText } from "@tauri-apps/plugin-clipboard-manager";
import { t } from "i18next";

interface ToastOptions {
  toast: (options: { title: string; status: "success" | "error" }) => void;
}

export const copyText = async (text: string, { toast }: ToastOptions) => {
  try {
    await writeText(text);
    toast({
      title: t("General.copy.toast.success"),
      status: "success",
    });
    return true;
  } catch (error) {
    logger.error("Copy failed:", error);
    toast({
      title: t("General.copy.toast.error"),
      status: "error",
    });
    return false;
  }
};

export const copyImage = async (
  img: string | Image | Uint8Array | ArrayBuffer | number[],
  { toast }: ToastOptions
) => {
  try {
    let image: Image;
    if (img instanceof Image) {
      image = img;
    } else if (typeof img === "string") {
      image = await Image.fromPath(img);
    } else {
      image = await Image.fromBytes(img as Uint8Array | ArrayBuffer | number[]);
    }
    await writeImage(image);
    toast({
      title: t("General.copy.toast.success"),
      status: "success",
    });
    return true;
  } catch (error) {
    logger.error("Copy image failed:", error);
    toast({
      title: t("General.copy.toast.error"),
      status: "error",
    });
    return false;
  }
};
