import {
  Button,
  FormControl,
  FormLabel,
  Input,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { JsonView } from "@/components/federation/json-view";
import { useManagementPreview } from "@/hooks/management-preview";

export function AuditAnchor({
  anchors,
  busy,
  request,
}: {
  anchors: Record<string, unknown>;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [id, setId] = useState("");
  const [digest, setDigest] = useState("");
  const {
    preview: review,
    failed: invalid,
    building,
    prepare,
    discard,
  } = useManagementPreview();
  return (
    <Section
      title={t("AuditAnchor.title")}
      description={t("AuditAnchor.description")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={3}>
        {Object.keys(anchors).length ? (
          <JsonView label={t("AuditAnchor.existing")} value={anchors} />
        ) : (
          <Text>{t("AuditAnchor.empty")}</Text>
        )}
        <FormControl>
          <FormLabel htmlFor="anchor-id">{t("AuditAnchor.id")}</FormLabel>
          <Input
            id="anchor-id"
            value={id}
            isDisabled={busy || building}
            onChange={(e) => {
              setId(e.target.value);
              discard();
            }}
          />
        </FormControl>
        <FormControl>
          <FormLabel htmlFor="anchor-digest">
            {t("AuditAnchor.digest")}
          </FormLabel>
          <Input
            id="anchor-digest"
            value={digest}
            isDisabled={busy || building}
            onChange={(e) => {
              setDigest(e.target.value);
              discard();
            }}
          />
        </FormControl>
        {invalid && <Text role="alert">{t("AuditAnchor.invalid")}</Text>}
        <Button
          isDisabled={busy || building || !id || !digest}
          onClick={() =>
            void prepare({
              operation: "build_anchor",
              id: id.trim(),
              digest: digest.trim(),
            })
          }
        >
          {t("IdentityFlow.review")}
        </Button>
        {review && (
          <>
            <Text>{t("AuditAnchor.confirm")}</Text>
            <JsonView label={t("AuditAnchor.review")} value={review} />
            <Button
              isDisabled={
                busy || building || Object.hasOwn(anchors, String(review.id))
              }
              onClick={() => void request(review)}
            >
              {t("AuditAnchor.submit")}
            </Button>
          </>
        )}
      </VStack>
    </Section>
  );
}
