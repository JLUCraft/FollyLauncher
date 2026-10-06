import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Text,
  Textarea,
  VStack,
} from "@chakra-ui/react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Section } from "@/components/common/section";
import { JsonView } from "@/components/federation/json-view";
import { useManagementPreview } from "@/hooks/management-preview";

type Replica = { rowId: string; id: string; peer: string; addresses: string };
export function BusinessAdmin({
  members,
  memberRoles,
  busy,
  request,
}: {
  members: Record<string, { peer: string; addresses: string[] }>;
  memberRoles: Record<string, string[]>;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [member, setMember] = useState("");
  const [roles, setRoles] = useState("");
  const [replicas, setReplicas] = useState<Replica[]>([]);
  const { preview, failed, building, prepare, discard, reset } =
    useManagementPreview();
  const lines = (text: string) =>
    text
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
  const change = (next: Replica[]) => {
    setReplicas(next);
    reset();
  };
  const edit = (rowId: string, patch: Partial<Replica>) =>
    change(replicas.map((r) => (r.rowId === rowId ? { ...r, ...patch } : r)));
  const locked = busy || building;
  return (
    <Section
      title={t("BusinessAdmin.title")}
      description={t("BusinessAdmin.description")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={3}>
        <FormControl>
          <FormLabel htmlFor="business-member">
            {t("IdentityManagement.fields.member")}
          </FormLabel>
          <Input
            id="business-member"
            value={member}
            isDisabled={locked}
            onChange={(e) => {
              setMember(e.target.value);
              discard();
            }}
          />
        </FormControl>
        {Object.hasOwn(memberRoles, member) && (
          <>
            <Text>
              {t("BusinessAdmin.currentRoles", {
                roles: memberRoles[member].join(", "),
              })}
            </Text>
            <Button
              isDisabled={locked}
              onClick={() => {
                setRoles(memberRoles[member].join("\n"));
                discard();
              }}
            >
              {t("BusinessAdmin.loadRoles")}
            </Button>
          </>
        )}
        <FormControl>
          <FormLabel htmlFor="business-roles">
            {t("BusinessAdmin.roles")}
          </FormLabel>
          <Textarea
            id="business-roles"
            value={roles}
            isDisabled={locked}
            onChange={(e) => {
              setRoles(e.target.value);
              discard();
            }}
          />
        </FormControl>
        <Text>{t("BusinessAdmin.rolesHint")}</Text>
        <Button
          isDisabled={locked || !member.trim()}
          onClick={() =>
            void prepare({
              operation: "build_roles",
              member,
              roles: roles.split("\n").filter((role) => role.trim()),
            })
          }
        >
          {t("BusinessAdmin.reviewRoles")}
        </Button>
        <Section
          title={t("BusinessAdmin.replicas")}
          description={t("BusinessAdmin.replicasHint")}
        >
          <VStack align="stretch" spacing={3}>
            {Object.keys(members).length ? (
              <JsonView label={t("BusinessAdmin.current")} value={members} />
            ) : (
              <Text>{t("BusinessAdmin.empty")}</Text>
            )}
            <Button
              isDisabled={locked || !Object.keys(members).length}
              onClick={() =>
                change(
                  Object.entries(members).map(([id, node]) => ({
                    rowId: crypto.randomUUID(),
                    id,
                    peer: node.peer,
                    addresses: node.addresses.join("\n"),
                  }))
                )
              }
            >
              {t("BusinessAdmin.load")}
            </Button>
            {replicas.map((row) => (
              <VStack align="stretch" key={row.rowId} spacing={2}>
                <HStack>
                  <Input
                    isDisabled={locked}
                    aria-label={t("BusinessAdmin.nodeId")}
                    placeholder={t("BusinessAdmin.nodeId")}
                    value={row.id}
                    onChange={(e) => edit(row.rowId, { id: e.target.value })}
                  />
                  <Input
                    isDisabled={locked}
                    aria-label={t("IdentityManagement.fields.audience")}
                    placeholder={t("IdentityManagement.fields.audience")}
                    value={row.peer}
                    onChange={(e) => edit(row.rowId, { peer: e.target.value })}
                  />
                </HStack>
                <Textarea
                  isDisabled={locked}
                  aria-label={t("BusinessAdmin.addresses")}
                  placeholder={t("BusinessAdmin.addresses")}
                  value={row.addresses}
                  onChange={(e) =>
                    edit(row.rowId, { addresses: e.target.value })
                  }
                />
                <Button
                  isDisabled={locked}
                  onClick={() =>
                    change(replicas.filter((r) => r.rowId !== row.rowId))
                  }
                >
                  {t("BusinessAdmin.remove")}
                </Button>
              </VStack>
            ))}
            <Button
              isDisabled={locked}
              onClick={() =>
                change([
                  ...replicas,
                  {
                    rowId: crypto.randomUUID(),
                    id: "",
                    peer: "",
                    addresses: "",
                  },
                ])
              }
            >
              {t("BusinessAdmin.add")}
            </Button>
            <Button
              isDisabled={locked || !replicas.length}
              onClick={() => {
                void prepare({
                  operation: "build_consensus",
                  members: replicas.map((row) => ({
                    id: row.id.trim(),
                    node: {
                      peer: row.peer.trim(),
                      addresses: lines(row.addresses),
                    },
                  })),
                });
              }}
            >
              {t("BusinessAdmin.reviewReplicas")}
            </Button>
          </VStack>
        </Section>
        {failed && <Text role="alert">{t("BusinessAdmin.invalid")}</Text>}
        {preview && (
          <>
            <Text>{t("BusinessAdmin.confirm")}</Text>
            <JsonView label={t("BusinessAdmin.preview")} value={preview} />
            <Button isDisabled={locked} onClick={() => void request(preview)}>
              {t("BusinessAdmin.submit")}
            </Button>
          </>
        )}
      </VStack>
    </Section>
  );
}
