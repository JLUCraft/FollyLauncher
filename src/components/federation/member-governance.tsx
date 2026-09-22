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
import { Section } from "@/components/common/section";
import { UnionState } from "@/services/union";

export function MemberGovernance({
  state,
  member,
  busy,
  request,
}: {
  state: UnionState;
  member?: string;
  busy: boolean;
  request: (action: Record<string, unknown>) => Promise<void>;
}) {
  const [category, setCategory] = useState("联赛"),
    [body, setBody] = useState("");
  return (
    <Section
      title="成员议事"
      description="查看提案、参与投票与提交申请"
      isAccordion
      initialIsOpen={false}
    >
      <VStack align="stretch" spacing={4}>
        {Object.entries(state.proposals ?? {}).map(([id, p]) => (
          <Box key={id} borderWidth="1px" borderRadius="md" p={3}>
            <HStack justify="space-between" align="center">
              <Text fontSize="sm">提案 {id}</Text>
              <Badge
                colorScheme={
                  p.executed ? "green" : p.cancelled ? "gray" : "yellow"
                }
              >
                {p.executed ? "已执行" : p.cancelled ? "已撤回" : "待处理"}
              </Badge>
            </HStack>
            <Text fontSize="xs" className="secondary-text" mt={1}>
              {Object.keys(p.approvals).length} 个社团同意
            </Text>
          </Box>
        ))}
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
                    {Object.values(b.votes).filter((v) => v === index).length}{" "}
                    票
                  </Text>
                </HStack>
              ))}
            </VStack>
            <Text fontSize="xs" className="secondary-text" mt={2}>
              截止 {new Date(b.closes_ms).toLocaleString()}
            </Text>
          </Box>
        ))}
        {!Object.keys(state.ballots ?? {}).length && <Text>暂无成员投票</Text>}
        {member && (
          <>
            <Divider />
            <FormControl>
              <FormLabel htmlFor="feedback-category">申请类别</FormLabel>
              <Input
                id="feedback-category"
                value={category}
                maxLength={128}
                onChange={(e) => setCategory(e.target.value)}
              />
            </FormControl>
            <FormControl>
              <FormLabel htmlFor="feedback-body">申请或建议</FormLabel>
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
              提交
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
                  {f.response ? "已回复" : "等待回复"}
                </Badge>
              </HStack>
              <Text fontSize="sm" mt={1}>
                {f.text}
              </Text>
              {f.response && (
                <Text fontSize="xs" className="secondary-text" mt={1}>
                  回复：{f.response}
                </Text>
              )}
            </Box>
          ))}
      </VStack>
    </Section>
  );
}
