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
          <Text key={id}>
            提案 {id} · {Object.keys(p.approvals).length} 个社团同意 ·{" "}
            {p.executed ? "已执行" : p.cancelled ? "已撤回" : "待处理"}
          </Text>
        ))}
        {Object.entries(state.ballots ?? {}).map(([id, b]) => (
          <VStack key={id} align="stretch">
            <Text fontWeight="bold">{b.title}</Text>
            {b.options.map((option, index) => (
              <HStack key={index}>
                <Button
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
                <Text>
                  {Object.values(b.votes).filter((v) => v === index).length} 票
                </Text>
              </HStack>
            ))}
            <Text fontSize="sm">
              截止 {new Date(b.closes_ms).toLocaleString()}
            </Text>
          </VStack>
        ))}
        {!Object.keys(state.ballots ?? {}).length && <Text>暂无成员投票</Text>}
        {member && (
          <>
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
            <VStack key={id} align="stretch">
              <Text>
                {f.category}：{f.text}
              </Text>
              <Text>{f.response ?? "等待回复"}</Text>
            </VStack>
          ))}
      </VStack>
    </Section>
  );
}
