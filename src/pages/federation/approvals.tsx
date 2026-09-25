import {
  Button,
  Checkbox,
  HStack,
  Input,
  Select,
  Tag,
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
import { OfficeFlow } from "@/services/office";
import { approvalControls } from "@/utils/federation/approval-controls";
import { encodeHex } from "@/utils/federation/hex";

export default function ApprovalsPage() {
  const { t } = useTranslation();
  const { data } = useOffice();
  const [pending, setPending] = useState(true);
  const flows = Object.entries(data?.state.flows || {}).filter(
    ([, v]) => !pending || ["active", "approved"].includes(v.status)
  );
  return (
    <OfficePage title={t("FederationNav.approvals")}>
      <Checkbox
        isChecked={pending}
        onChange={(e) => setPending(e.target.checked)}
      >
        {t("Office.pending")}
      </Checkbox>
      {data && flows.length === 0 && <Empty description={t("Office.empty")} />}
      {flows.map(([id, flow]) => (
        <ApprovalCard key={id} id={id} flow={flow} />
      ))}
    </OfficePage>
  );
}

function ApprovalCard({ id, flow }: { id: string; flow: OfficeFlow }) {
  const { t } = useTranslation();
  const { data, member, isAdmin } = useOffice();
  const request = useOfficeRequest();
  const { busy, pendingKey, act } = request;
  const [comment, setComment] = useState("");
  const [to, setTo] = useState("");
  const [hashing, setHashing] = useState(false);
  const vote = async (instance: string, reject: boolean, key: string) => {
    setHashing(true);
    try {
      const bytes = await crypto.subtle.digest(
        "SHA-256",
        new TextEncoder().encode(comment)
      );
      const hash = encodeHex(new Uint8Array(bytes));
      const success = await act(
        reject
          ? { type: "flows.reject", instance, reason: hash }
          : { type: "flows.approve", instance, comment_hash: hash },
        key
      );
      if (success) setComment("");
    } finally {
      setHashing(false);
    }
  };
  const template =
    data?.state.flow_templates[flow.template]?.versions[
      flow.template_epoch - 1
    ];
  const stage = template?.stages[flow.current];
  const controls = approvalControls(
    stage?.approver,
    flow.records[flow.current],
    data?.identity || { admins: [], members: {} },
    member
  );
  return (
    <AdvancedCard>
      <VStack align="stretch" spacing={3}>
        <HStack justify="space-between">
          <Text fontWeight="semibold">{template?.name || flow.template}</Text>
          <Tag>{t(`Office.status.${flow.status}`)}</Tag>
        </HStack>
        <HStack>
          <Text fontSize="sm">{t("Office.proposer")}</Text>
          <HashChip value={flow.proposer} />
        </HStack>
        <Text>{stage?.name}</Text>
        <Text fontSize="sm">
          {t("Office.event")}:{" "}
          {data?.state.events?.[String(flow.payload.id || "")]?.title ||
            t("Office.empty")}
        </Text>
        {flow.records
          .flatMap((record) => record.ai_runs || [])
          .map((run) => (
            <Section
              key={run.job}
              title={t("Office.aiResult")}
              isAccordion
              initialIsOpen={false}
            >
              {run.output ? (
                <HashChip value={run.output} />
              ) : (
                <Text>
                  {t(run.error ? "Office.error" : "Office.status.active")}
                </Text>
              )}
            </Section>
          ))}
        {member &&
          flow.status === "active" &&
          (controls.approve || controls.reject || controls.delegate) && (
            <>
              <Input
                aria-label={t("Office.comment")}
                placeholder={t("Office.comment")}
                value={comment}
                onChange={(e) => setComment(e.target.value)}
              />
              <Text fontSize="xs">{t("Office.commentHint")}</Text>
              <HStack>
                <Button
                  isDisabled={
                    busy || hashing || !comment.trim() || !controls.approve
                  }
                  isLoading={busy && pendingKey === `flows:${id}:approve`}
                  onClick={() => void vote(id, false, `flows:${id}:approve`)}
                >
                  {t("Office.approve")}
                </Button>
                <Button
                  isDisabled={
                    busy || hashing || !comment.trim() || !controls.reject
                  }
                  isLoading={busy && pendingKey === `flows:${id}:reject`}
                  onClick={() => void vote(id, true, `flows:${id}:reject`)}
                >
                  {t("Office.reject")}
                </Button>
              </HStack>
              {controls.delegate && (
                <HStack>
                  <Select
                    aria-label={t("Office.assignee")}
                    placeholder={t("Office.assignee")}
                    value={to}
                    onChange={(e) => setTo(e.target.value)}
                  >
                    {Object.entries(data?.identity.members || {})
                      .filter(
                        ([id, m]) =>
                          id !== member &&
                          m.club === data?.identity.members[member]?.club
                      )
                      .map(([id]) => (
                        <option key={id} value={id}>
                          {id}
                        </option>
                      ))}
                  </Select>
                  <Button
                    isDisabled={busy || !to || stage?.approver.type === "ai"}
                    isLoading={busy && pendingKey === `flows:${id}:delegate`}
                    onClick={() =>
                      void act(
                        {
                          type: "flows.delegate",
                          instance: id,
                          to,
                        },
                        `flows:${id}:delegate`
                      ).then((success) => {
                        if (success) setTo("");
                      })
                    }
                  >
                    {t("Office.delegate")}
                  </Button>
                </HStack>
              )}
            </>
          )}
        {member && flow.status === "active" && (
          <Button
            isDisabled={busy}
            isLoading={busy && pendingKey === `flows:${id}:poke`}
            onClick={() =>
              void act({ type: "flows.poke", instance: id }, `flows:${id}:poke`)
            }
          >
            {t("Office.poke")}
          </Button>
        )}
        {(isAdmin || member === flow.proposer) &&
          flow.status === "approved" && (
            <Button
              isDisabled={busy}
              isLoading={busy && pendingKey === `flows:${id}:execute`}
              onClick={() =>
                void act(
                  { type: "flows.execute", instance: id },
                  `flows:${id}:execute`
                )
              }
            >
              {t("Office.execute")}
            </Button>
          )}
      </VStack>
    </AdvancedCard>
  );
}
