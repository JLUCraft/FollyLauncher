import {
  Badge,
  Box,
  Button,
  Divider,
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
import { UnionState } from "@/services/union";

export function MemberGovernance({
  state,
  member,
  identity,
  busy,
  request,
}: {
  state: UnionState;
  member?: string;
  identity: string;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [category, setCategory] = useState(
      t("MemberGovernance.defaultCategory")
    ),
    [body, setBody] = useState("");
  return (
    <Section
      title={t("MemberGovernance.title")}
      description={t("MemberGovernance.description")}
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={4}>
        {Object.entries(state.proposals ?? {}).map(([id, p]) => {
          const expired = !!state.clock_ms && state.clock_ms >= p.expires_ms;
          const settled = p.executed || p.cancelled || expired;
          const myClub = member ? state.members[member]?.club : undefined;
          const mySchoolDevices = myClub
            ? state.council?.schools[myClub]
            : undefined;
          const threshold = state.council
            ? p.action.type === "update_council"
              ? state.council.change_quorum
              : state.council.quorum
            : 0;
          const approvedByMySchool =
            !!myClub && p.approvals[myClub] !== undefined;
          const canApprove =
            !settled &&
            !!mySchoolDevices?.includes(identity) &&
            !approvedByMySchool;
          const canExecute =
            !settled && Object.keys(p.approvals).length >= threshold;
          const canCancel = !settled && p.proposer === identity;
          return (
            <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
              <HStack justify="space-between" align="center">
                <Text fontSize="sm">
                  {t("MemberGovernance.proposalId", { id })}
                </Text>
                <Badge
                  colorScheme={
                    p.executed
                      ? "green"
                      : p.cancelled
                        ? "gray"
                        : expired
                          ? "gray"
                          : "yellow"
                  }
                >
                  {p.executed
                    ? t("MemberGovernance.proposalStatus.executed")
                    : p.cancelled
                      ? t("MemberGovernance.proposalStatus.cancelled")
                      : expired
                        ? t("MemberGovernance.proposalStatus.expired")
                        : t("MemberGovernance.proposalStatus.pending")}
                </Badge>
              </HStack>
              <Text fontSize="xs" className="secondary-text" mt={1}>
                {t("MemberGovernance.approvalsProgress", {
                  count: Object.keys(p.approvals).length,
                  needed: threshold,
                })}
              </Text>
              {(canApprove || canExecute || canCancel) && (
                <HStack spacing={2} mt={2}>
                  {canApprove && (
                    <Button
                      size="sm"
                      colorScheme="blue"
                      isDisabled={busy}
                      onClick={() =>
                        void request({ type: "approve_proposal", id })
                      }
                    >
                      {t("MemberGovernance.approve")}
                    </Button>
                  )}
                  {canExecute && (
                    <Button
                      size="sm"
                      colorScheme="green"
                      isDisabled={busy}
                      onClick={() =>
                        void request({ type: "execute_proposal", id })
                      }
                    >
                      {t("MemberGovernance.execute")}
                    </Button>
                  )}
                  {canCancel && (
                    <Button
                      size="sm"
                      isDisabled={busy}
                      onClick={() =>
                        void request({ type: "cancel_proposal", id })
                      }
                    >
                      {t("MemberGovernance.cancel")}
                    </Button>
                  )}
                </HStack>
              )}
            </Box>
          );
        })}
        {Object.entries(state.ballots ?? {}).map(([id, b]) => (
          <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
            <Text fontWeight="bold">{b.title}</Text>
            <VStack align="stretch" mt={2} spacing={2}>
              {b.options.map((option, index) => (
                <HStack key={index}>
                  <Button
                    size="sm"
                    maxW="100%"
                    whiteSpace="normal"
                    textAlign="left"
                    isDisabled={
                      busy ||
                      !member ||
                      !b.electorate.includes(member) ||
                      b.votes[member] !== undefined ||
                      Date.now() >= b.closes_ms
                    }
                    onClick={() =>
                      void request({ type: "member_vote", id, option: index })
                    }
                  >
                    {option}
                  </Button>
                  <Text fontSize="sm" className="secondary-text">
                    {t("MemberGovernance.votes", {
                      count: Object.values(b.votes).filter((v) => v === index)
                        .length,
                    })}
                  </Text>
                </HStack>
              ))}
            </VStack>
            <Text fontSize="xs" className="secondary-text" mt={2}>
              {t("MemberGovernance.closesAt", {
                time: new Date(b.closes_ms).toLocaleString(),
              })}
            </Text>
          </Box>
        ))}
        {!Object.keys(state.ballots ?? {}).length && (
          <Text>{t("MemberGovernance.noBallots")}</Text>
        )}
        {member && (
          <>
            <Divider />
            <FormControl>
              <FormLabel htmlFor="feedback-category">
                {t("MemberGovernance.categoryLabel")}
              </FormLabel>
              <Input
                id="feedback-category"
                value={category}
                maxLength={128}
                onChange={(e) => setCategory(e.target.value)}
              />
            </FormControl>
            <FormControl>
              <FormLabel htmlFor="feedback-body">
                {t("MemberGovernance.bodyLabel")}
              </FormLabel>
              <Textarea
                id="feedback-body"
                value={body}
                maxLength={4096}
                onChange={(e) => setBody(e.target.value)}
              />
            </FormControl>
            <Button
              alignSelf="flex-start"
              isDisabled={busy || !body.trim() || !category.trim()}
              onClick={() =>
                void request({
                  type: "submit_feedback",
                  id: crypto.randomUUID(),
                  category,
                  text: body,
                })
              }
            >
              {t("MemberGovernance.submit")}
            </Button>
          </>
        )}
        {Object.entries(state.feedback ?? {})
          .filter(([, f]) => f.member === member)
          .map(([id, f]) => (
            <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
              <HStack justify="space-between" align="center">
                <Text fontSize="sm" fontWeight="semibold">
                  {f.category}
                </Text>
                <Badge colorScheme={f.response ? "green" : "gray"}>
                  {f.response
                    ? t("MemberGovernance.feedbackReplied")
                    : t("MemberGovernance.feedbackPending")}
                </Badge>
              </HStack>
              <Text fontSize="sm" mt={1}>
                {f.text}
              </Text>
              {f.response && (
                <Text fontSize="xs" className="secondary-text" mt={1}>
                  {t("MemberGovernance.feedbackResponse", {
                    response: f.response,
                  })}
                </Text>
              )}
            </Box>
          ))}
      </VStack>
    </Section>
  );
}
