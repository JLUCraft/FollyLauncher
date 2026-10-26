import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { CommonIconButton } from "@/components/common/common-icon-button";
import {
  OptionItemGroup,
  OptionItemGroupProps,
} from "@/components/common/option-item";

const HelpSettingsPage = () => {
  const { t } = useTranslation();
  const helpSettingGroups: (OptionItemGroupProps & { id: string })[] = [
    {
      id: "launcher",
      items: [
        {
          title: t("HelpSettingsPage.top.settings.LauncherDocs.title"),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.top.settings.LauncherDocs.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              h={18}
              onClick={() =>
                openUrl(t("HelpSettingsPage.top.settings.LauncherDocs.url"))
              }
            />
          ),
        },
        {
          title: t("HelpSettingsPage.top.settings.UserGroup.title"),
          description: t("HelpSettingsPage.top.settings.UserGroup.description"),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.top.settings.UserGroup.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              h={18}
              onClick={() =>
                openUrl(t("HelpSettingsPage.top.settings.UserGroup.url"))
              }
            />
          ),
        },
      ],
    },
    {
      id: "minecraft",
      title: t("HelpSettingsPage.minecraft.title"),
      items: [
        {
          title: t("HelpSettingsPage.minecraft.settings.mcWiki.title"),
          description: t(
            "HelpSettingsPage.minecraft.settings.mcWiki.description"
          ),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.minecraft.settings.mcWiki.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() =>
                openUrl(t("HelpSettingsPage.minecraft.settings.mcWiki.url"))
              }
            />
          ),
        },
        {
          title: t("HelpSettingsPage.minecraft.settings.mcMod.title"),
          description: t(
            "HelpSettingsPage.minecraft.settings.mcMod.description"
          ),
          children: (
            <CommonIconButton
              label="https://www.mcmod.cn/"
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() => openUrl("https://www.mcmod.cn/")}
            />
          ),
        },
        {
          title: t("HelpSettingsPage.minecraft.settings.curseforge.title"),
          description: t(
            "HelpSettingsPage.minecraft.settings.curseforge.description"
          ),
          children: (
            <CommonIconButton
              label="https://www.curseforge.com/minecraft"
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() => openUrl("https://www.curseforge.com/minecraft")}
            />
          ),
        },
        {
          title: t("HelpSettingsPage.minecraft.settings.modrinth.title"),
          description: t(
            "HelpSettingsPage.minecraft.settings.modrinth.description"
          ),
          children: (
            <CommonIconButton
              label="https://modrinth.com/discover/mods"
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() => openUrl("https://modrinth.com/discover/mods")}
            />
          ),
        },
      ],
    },
    {
      id: "community",
      title: t("HelpSettingsPage.community.title"),
      items: [
        {
          title: t("HelpSettingsPage.community.settings.JLUCraft.title"),
          description: t(
            "HelpSettingsPage.community.settings.JLUCraft.description"
          ),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.community.settings.JLUCraft.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() =>
                openUrl(t("HelpSettingsPage.community.settings.JLUCraft.url"))
              }
            />
          ),
        },
        {
          title: t("HelpSettingsPage.community.settings.MUA.title"),
          description: t("HelpSettingsPage.community.settings.MUA.description"),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.community.settings.MUA.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              onClick={() =>
                openUrl(t("HelpSettingsPage.community.settings.MUA.url"))
              }
            />
          ),
        },
      ],
    },
    {
      id: "upstream",
      title: t("HelpSettingsPage.upstream.title"),
      items: [
        {
          title: t("HelpSettingsPage.upstream.settings.SJMC.title"),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.upstream.settings.SJMC.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              h={18}
              onClick={() =>
                openUrl(t("HelpSettingsPage.upstream.settings.SJMC.url"))
              }
            />
          ),
        },
        {
          title: t("HelpSettingsPage.upstream.settings.LauncherDocs.title"),
          children: (
            <CommonIconButton
              label={t("HelpSettingsPage.upstream.settings.LauncherDocs.url")}
              icon="external"
              withTooltip
              tooltipPlacement="bottom-end"
              size="xs"
              h={18}
              onClick={() =>
                openUrl(
                  t("HelpSettingsPage.upstream.settings.LauncherDocs.url")
                )
              }
            />
          ),
        },
      ],
    },
  ];

  return (
    <>
      {helpSettingGroups.map(({ id, ...group }) => (
        <OptionItemGroup key={id} {...group} />
      ))}
    </>
  );
};

export default HelpSettingsPage;
