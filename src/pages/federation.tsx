import {
  Box,
  Button,
  Flex,
  FormControl,
  FormLabel,
  HStack,
  Heading,
  Input,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
import { useEffect, useState } from "react";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { MemberGovernance } from "@/components/federation/member-governance";
import { UnionService, UnionState } from "@/services/union";

const PHASE_LABELS: Record<string, string> = {
  Registration: "报名中",
  Playing: "进行中",
  Finished: "已结束",
};

const Scoreboard = ({
  title,
  scores,
  emptyText,
}: {
  title: string;
  scores: Record<string, number>;
  emptyText: string;
}) => (
  <Box>
    <Text fontWeight="bold" fontSize="sm" mb={2}>
      {title}
    </Text>
    {Object.keys(scores).length === 0 ? (
      <Text fontSize="sm" className="secondary-text">
        {emptyText}
      </Text>
    ) : (
      <VStack spacing={1} align="stretch">
        {Object.entries(scores)
          .sort((a, b) => b[1] - a[1])
          .map(([id, score], index) => (
            <Flex key={id} fontSize="sm" align="center">
              <Text
                w={8}
                flexShrink={0}
                fontWeight={index < 3 ? "bold" : "normal"}
                color={index < 3 ? "blue.500" : "gray.500"}
              >
                {index + 1}
              </Text>
              <Text flex="1" overflowWrap="anywhere">
                {id}
              </Text>
              <Text fontWeight="medium" flexShrink={0}>
                {score} 分
              </Text>
            </Flex>
          ))}
      </VStack>
    )}
  </Box>
);

export default function FederationPage() {
  const [gameLink, setGameLink] = useState("");
  const [gameAddress, setGameAddress] = useState<string | null>(null);
  const [gameBusy, setGameBusy] = useState(false);
  useEffect(() => {
    let active = true;
    const refresh = () => {
      void UnionService.gameConnection()
        .then((value) => {
          if (active) setGameAddress(value);
        })
        .catch(() => {});
    };
    refresh();
    const timer = setInterval(refresh, 3000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);
  const handleGameConnection = async () => {
    setGameBusy(true);
    setError("");
    try {
      if (gameAddress) {
        await UnionService.disconnectGame();
        setGameAddress(null);
      } else {
        setGameAddress(await UnionService.connectGame(gameLink));
      }
    } catch {
      setError("游戏连接未完成，请检查链接或联系组织者");
    } finally {
      setGameBusy(false);
    }
  };
  const [address, setAddress] = useState("");
  const [peer, setPeer] = useState("");
  const [identity, setIdentity] = useState("");
  const [state, setState] = useState<UnionState>();
  const [teamName, setTeamName] = useState("");
  const [invitee, setInvitee] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const handleRequest = async (action?: Record<string, unknown>) => {
    setBusy(true);
    setError("");
    try {
      setIdentity(await UnionService.identity());
      setState(
        await UnionService.request(address, peer, state?.revision, action)
      );
    } catch {
      setError("操作未完成，请检查连接与报名状态后重试");
    } finally {
      setBusy(false);
    }
  };
  const member = Object.entries(state?.members || {}).find(
    ([, m]) => m.device === identity
  )?.[0];
  const leagues = Object.entries(state?.leagues || {});
  return (
    <VStack p={6} spacing={6} align="stretch">
      <Box>
        <Heading size="lg">高校联赛</Heading>
        <Text fontSize="sm" className="secondary-text" mt={1}>
          连接联盟后查看联赛、创建队伍、邀请队友并提交报名。
        </Text>
      </Box>

      {state && (
        <MemberGovernance
          state={state}
          member={member}
          busy={busy}
          request={handleRequest}
        />
      )}

      <Section title="比赛连接" description="粘贴组织者提供的比赛连接链接">
        <VStack align="stretch" spacing={3}>
          <FormControl>
            <FormLabel htmlFor="game-link">连接链接</FormLabel>
            <Input
              id="game-link"
              value={gameLink}
              isDisabled={gameBusy || !!gameAddress}
              onChange={(e) => setGameLink(e.target.value)}
            />
          </FormControl>
          {gameAddress && (
            <Text fontSize="sm">
              游戏服务器地址：{gameAddress}
              。从启动器启动 Minecraft 26.3 后会自动加入，请保持启动器运行。
            </Text>
          )}
          <Button
            colorScheme="blue"
            isLoading={gameBusy}
            isDisabled={!gameAddress && !gameLink}
            onClick={() => void handleGameConnection()}
          >
            {gameAddress ? "断开游戏连接" : "建立游戏连接"}
          </Button>
        </VStack>
      </Section>

      <Section
        title="连接设置"
        description="联盟连接地址与联盟身份由组织者提供"
        isAccordion
        initialIsOpen={!state}
      >
        <VStack spacing={3} align="stretch">
          <FormControl>
            <FormLabel htmlFor="federation-address">联盟连接地址</FormLabel>
            <Input
              id="federation-address"
              isDisabled={busy}
              value={address}
              onChange={(e) => {
                setAddress(e.target.value);
                setState(undefined);
              }}
            />
          </FormControl>
          <FormControl>
            <FormLabel htmlFor="federation-peer">联盟身份</FormLabel>
            <Input
              id="federation-peer"
              isDisabled={busy}
              value={peer}
              onChange={(e) => {
                setPeer(e.target.value);
                setState(undefined);
              }}
            />
          </FormControl>
          <HStack justify="flex-end">
            <Button
              colorScheme="blue"
              isLoading={busy}
              isDisabled={!address || !peer}
              onClick={() => void handleRequest()}
            >
              连接 / 刷新
            </Button>
          </HStack>
          {identity && (
            <VStack spacing={1} align="start" fontSize="sm">
              <Text overflowWrap="anywhere">
                我的设备：<Text as="span">{identity}</Text>
              </Text>
              {member && (
                <Text>
                  我的编号：<Text as="span">{member}</Text>
                  {state?.members[member]?.club && (
                    <Text as="span" className="secondary-text">
                      （{state.members[member].club}）
                    </Text>
                  )}
                </Text>
              )}
            </VStack>
          )}
        </VStack>
      </Section>

      {error && (
        <Text role="alert" color="red.500" fontSize="sm">
          {error}
        </Text>
      )}

      {!state ? (
        <Empty
          size="md"
          description="完成连接设置后，这里会显示联赛列表、队伍与积分榜。"
        />
      ) : leagues.length === 0 ? (
        <Empty size="md" description="当前联盟暂无联赛。" />
      ) : (
        leagues.map(([league, value]) => (
          <AdvancedCard key={league} level="front">
            <VStack spacing={4} align="stretch">
              <Flex wrap="wrap" gap={2} align="center" justify="space-between">
                <Heading size="md">{league}</Heading>
                <HStack spacing={2}>
                  <Tag
                    colorScheme={
                      value.phase === "Registration"
                        ? "blue"
                        : value.phase === "Playing"
                          ? "green"
                          : "gray"
                    }
                  >
                    {PHASE_LABELS[value.phase] || value.phase}
                  </Tag>
                  <Tag colorScheme="gray">每队 {value.rules.seats} 人</Tag>
                  <Tag colorScheme={value.rules.cross_club ? "green" : "gray"}>
                    {value.rules.cross_club ? "允许跨校" : "同校组队"}
                  </Tag>
                </HStack>
              </Flex>

              {value.phase === "Registration" && member && (
                <HStack spacing={2}>
                  <Input
                    aria-label="队伍名称"
                    placeholder="新队伍名称"
                    value={teamName}
                    onChange={(e) => setTeamName(e.target.value)}
                  />
                  <Button
                    colorScheme="blue"
                    flexShrink={0}
                    isDisabled={busy || !teamName}
                    onClick={() =>
                      void handleRequest({
                        type: "create_team",
                        league,
                        team: teamName,
                      })
                    }
                  >
                    创建队伍
                  </Button>
                </HStack>
              )}

              <Box>
                <Text fontWeight="bold" fontSize="sm" mb={2}>
                  参赛队伍
                </Text>
                {Object.keys(value.teams).length === 0 ? (
                  <Text fontSize="sm" className="secondary-text">
                    暂无队伍。报名阶段，已加入联盟的队员可以创建队伍。
                  </Text>
                ) : (
                  Object.entries(value.teams).map(([team, roster]) => (
                    <Box
                      key={team}
                      py={3}
                      borderBottomWidth="1px"
                      borderColor="var(--chakra-colors-chakra-border-color)"
                      _last={{ borderBottomWidth: 0 }}
                    >
                      <Flex wrap="wrap" gap={2} align="center">
                        <Text fontWeight="medium">{team}</Text>
                        <Tag colorScheme="gray">{roster.members.length} 人</Tag>
                        <Tag colorScheme={roster.registered ? "green" : "gray"}>
                          {roster.registered ? "已提交报名" : "未提交报名"}
                        </Tag>
                        <HStack spacing={2} ml="auto">
                          {value.phase === "Registration" &&
                            member &&
                            roster.invited.includes(member) && (
                              <Button
                                colorScheme="blue"
                                isDisabled={busy}
                                onClick={() =>
                                  void handleRequest({
                                    type: "accept",
                                    league,
                                    team,
                                  })
                                }
                              >
                                接受邀请
                              </Button>
                            )}
                          {value.phase === "Registration" &&
                            roster.captain === member && (
                              <>
                                <Button
                                  colorScheme="green"
                                  isDisabled={busy}
                                  onClick={() =>
                                    void handleRequest({
                                      type: "register",
                                      league,
                                      team,
                                    })
                                  }
                                >
                                  提交报名
                                </Button>
                              </>
                            )}
                        </HStack>
                      </Flex>
                      {value.phase === "Registration" &&
                        roster.captain === member && (
                          <HStack spacing={2} mt={2}>
                            <Input
                              aria-label={`邀请成员加入 ${team}`}
                              placeholder="邀请成员的联盟编号"
                              value={invitee}
                              onChange={(e) => setInvitee(e.target.value)}
                            />
                            <Button
                              flexShrink={0}
                              isDisabled={busy || !invitee}
                              onClick={() =>
                                void handleRequest({
                                  type: "invite",
                                  league,
                                  team,
                                  member: invitee,
                                })
                              }
                            >
                              邀请成员
                            </Button>
                          </HStack>
                        )}
                    </Box>
                  ))
                )}
              </Box>

              <Flex
                gap={8}
                direction={{ base: "column", md: "row" }}
                align="flex-start"
              >
                <Box flex="1" minW={0}>
                  <Scoreboard
                    title="队伍积分榜"
                    scores={value.team_scores}
                    emptyText="队伍积分榜暂无数据。"
                  />
                </Box>
                <Box flex="1" minW={0}>
                  <Scoreboard
                    title="玩家积分榜"
                    scores={value.player_scores}
                    emptyText="玩家积分榜暂无数据。"
                  />
                </Box>
              </Flex>
            </VStack>
          </AdvancedCard>
        ))
      )}
    </VStack>
  );
}
