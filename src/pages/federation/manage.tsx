import { Text } from "@chakra-ui/react";
import { useTranslation } from "react-i18next";
import Empty from "@/components/common/empty";
import { AdminConsole } from "@/components/federation/admin-console";
import { AuditAnchor } from "@/components/federation/audit-anchor";
import { BusinessAdmin } from "@/components/federation/business-admin";
import { ConnectionRequired } from "@/components/federation/connection-required";
import { useFederation, useLeagueRequest } from "@/contexts/federation";

export default function ManagePage() {
  const { t } = useTranslation();
  const { state, isAdmin } = useFederation();
  const { busy, error, handleRequest } = useLeagueRequest();
  if (!state) return <ConnectionRequired />;
  if (!isAdmin) return <Empty description={t("FederationNav.restricted")} />;
  return (
    <>
      {error && (
        <Text role="alert" color="red.600">
          {error}
        </Text>
      )}
      <AuditAnchor
        anchors={state.digest_anchors || {}}
        busy={busy}
        request={handleRequest}
      />
      <BusinessAdmin
        members={state.consensus_members || {}}
        memberRoles={Object.fromEntries(
          Object.entries(state.members).map(([id, member]) => [
            id,
            member.roles || [],
          ])
        )}
        busy={busy}
        request={handleRequest}
      />
      <AdminConsole state={state} busy={busy} request={handleRequest} />
    </>
  );
}
