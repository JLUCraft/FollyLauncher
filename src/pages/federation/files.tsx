import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { HashChip } from "@/components/federation/hash-chip";
import { OfficePage } from "@/components/federation/office-page";
import { useOffice, useOfficeRequest } from "@/contexts/office";
import { OfficeService } from "@/services/office";

export default function FilesPage() {
  const { t } = useTranslation();
  const { target, data, member } = useOffice();
  const request = useOfficeRequest();
  const { busy, pendingKey, run } = request;
  const [branch, setBranch] = useState("");
  const [path, setPath] = useState("");
  const [message, setMessage] = useState("");
  const [file, setFile] = useState<File | null>(null);
  const [fileEpoch, setFileEpoch] = useState(0);
  const [revision, setRevision] = useState("");
  const [query, setQuery] = useState("");
  const [downloads, setDownloads] = useState<Set<string>>(() => new Set());
  const [downloadError, setDownloadError] = useState(false);
  const head = data?.state.files?.branches[branch]?.commit;
  const history: string[] = [];
  let cid = head;
  while (cid && !history.includes(cid)) {
    history.push(cid);
    cid = data?.commits[cid]?.parent?.["/"];
  }
  const selected = history.includes(revision) ? revision : head;
  const commit = selected ? data?.commits[selected] : undefined;
  const handleUpload = () =>
    run(async (target) => {
      if (!file || !data || file.size > data.max_file_bytes)
        throw new Error("FILE_SIZE");
      await OfficeService.upload(target, {
        branch,
        path,
        message,
        bytes: new Uint8Array(await file.arrayBuffer()),
      });
    }, "files:upload").then((success) => {
      if (success) {
        setFile(null);
        setPath("");
        setMessage("");
        setFileEpoch((epoch) => epoch + 1);
      }
    });
  const handleDownload = async (cid: string, name: string) => {
    const downloadKey = JSON.stringify([selected, name, cid]);
    if (!target) return;
    setDownloads((current) => new Set(current).add(downloadKey));
    setDownloadError(false);
    try {
      const bytes = await OfficeService.fetch(target, cid);
      const url = URL.createObjectURL(new Blob([new Uint8Array(bytes)]));
      const link = document.createElement("a");
      link.href = url;
      link.download = name.split("/").at(-1) || name;
      link.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch {
      setDownloadError(true);
    } finally {
      setDownloads((current) => {
        const next = new Set(current);
        next.delete(downloadKey);
        return next;
      });
    }
  };
  const restore = (cid: string, path: string) =>
    run(async (target) => {
      const bytes = await OfficeService.fetch(target, cid);
      await OfficeService.upload(target, { branch, path, message, bytes });
    }, `files:restore:${cid}`);
  return (
    <OfficePage title={t("FederationNav.files")}>
      <Text fontSize="sm">
        {t("Office.fileLimits", {
          bytes: data?.max_file_bytes,
          paths: data?.max_commit_paths,
        })}
      </Text>
      <FormControl>
        <FormLabel htmlFor="file-branch">{t("Office.branch")}</FormLabel>
        <Input
          id="file-branch"
          value={branch}
          onChange={(e) => {
            setBranch(e.target.value);
            setRevision("");
          }}
          isDisabled={busy}
        />
      </FormControl>
      <Select
        aria-label={t("Office.branch")}
        placeholder={t("Office.chooseBranch")}
        value={head ? branch : ""}
        onChange={(e) => {
          setBranch(e.target.value);
          setRevision("");
        }}
        isDisabled={busy}
      >
        {Object.keys(data?.state.files?.branches || {}).map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </Select>
      <FormControl>
        <FormLabel htmlFor="file-message">{t("Office.message")}</FormLabel>
        <Input
          id="file-message"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          isDisabled={busy}
        />
      </FormControl>
      {member && (
        <Section title={t("Office.upload")} isAccordion initialIsOpen={false}>
          <VStack align="stretch" spacing={3}>
            <FormControl>
              <FormLabel htmlFor="file-path">{t("Office.path")}</FormLabel>
              <Input
                id="file-path"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                isDisabled={busy}
              />
            </FormControl>
            <Input
              key={fileEpoch}
              type="file"
              aria-label={t("Office.upload")}
              isDisabled={busy}
              onChange={(e) => {
                const f = e.target.files?.[0] || null;
                setFile(f);
                if (f) setPath(f.name);
              }}
            />
            {file && data && file.size > data.max_file_bytes && (
              <Text role="alert">{t("Office.tooLarge")}</Text>
            )}
            <Button
              isDisabled={
                busy ||
                !file ||
                !branch ||
                !path ||
                !message ||
                !data ||
                file.size > data.max_file_bytes
              }
              isLoading={busy && pendingKey === "files:upload"}
              onClick={() => void handleUpload()}
            >
              {t("Office.upload")}
            </Button>
          </VStack>
        </Section>
      )}
      {head && (
        <>
          <Select
            aria-label={t("Office.history")}
            value={selected}
            onChange={(e) => setRevision(e.target.value)}
          >
            {history.map((cid) => (
              <option key={cid} value={cid}>
                {data?.commits[cid]?.message} · {cid.slice(-8)}
              </option>
            ))}
          </Select>
          <HashChip value={selected || ""} />
        </>
      )}
      <Input
        aria-label={t("Office.searchFiles")}
        placeholder={t("Office.searchFiles")}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
      />
      {data &&
        (!commit ||
          !Object.keys(commit.files).some((path) =>
            path.toLocaleLowerCase().includes(query.toLocaleLowerCase())
          )) && <Empty description={t("Office.empty")} />}
      {Object.entries(commit?.files || {})
        .filter(([path]) =>
          path.toLocaleLowerCase().includes(query.toLocaleLowerCase())
        )
        .map(([path, link]) => (
          <AdvancedCard key={path}>
            <VStack align="stretch">
              <Text>{path}</Text>
              <HashChip value={link["/"]} />
              <HStack>
                <Button
                  isLoading={downloads.has(
                    JSON.stringify([selected, path, link["/"]])
                  )}
                  onClick={() => void handleDownload(link["/"], path)}
                >
                  {t("Office.download")}
                </Button>
                {member && selected !== head && (
                  <Button
                    isDisabled={busy || !message}
                    isLoading={
                      busy && pendingKey === `files:restore:${link["/"]}`
                    }
                    onClick={() => void restore(link["/"], path)}
                  >
                    {t("Office.restore")}
                  </Button>
                )}
              </HStack>
            </VStack>
          </AdvancedCard>
        ))}
      {downloadError && <Text role="alert">{t("Office.error")}</Text>}
    </OfficePage>
  );
}
