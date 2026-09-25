import {
  Button,
  FormControl,
  FormLabel,
  HStack,
  Input,
  Select,
  Tag,
  Text,
  VStack,
} from "@chakra-ui/react";
import Link from "next/link";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import AdvancedCard from "@/components/common/advanced-card";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import { HashChip } from "@/components/federation/hash-chip";
import { OfficePage } from "@/components/federation/office-page";
import { useOffice, useOfficeRequest } from "@/contexts/office";
import { OfficeEvent, OfficeTask } from "@/services/office";

export default function EventsPage() {
  const { t } = useTranslation();
  const { data, member } = useOffice();
  const request = useOfficeRequest();
  const { busy, pendingKey, act } = request;
  const [title, setTitle] = useState("");
  const [brief, setBrief] = useState("");
  const [opens, setOpens] = useState("");
  const [closes, setCloses] = useState("");
  const [starts, setStarts] = useState("");
  const eventWindow = {
    opens_ms: Date.parse(opens),
    closes_ms: Date.parse(closes),
    starts_ms: Date.parse(starts),
  };
  const validWindow =
    Object.values(eventWindow).every(Number.isFinite) &&
    eventWindow.opens_ms < eventWindow.closes_ms &&
    eventWindow.closes_ms <= eventWindow.starts_ms;
  return (
    <OfficePage title={t("FederationNav.events")}>
      {member && (
        <Section title={t("Office.create")} isAccordion initialIsOpen={false}>
          <VStack align="stretch" spacing={3}>
            {[
              ["title", title, setTitle],
              ["opens", opens, setOpens],
              ["closes", closes, setCloses],
              ["starts", starts, setStarts],
            ].map(([key, value, setter]) => (
              <FormControl key={key as string}>
                <FormLabel htmlFor={`event-${key}`}>
                  {t(`Office.${key}`)}
                </FormLabel>
                <Input
                  id={`event-${key}`}
                  type={
                    ["opens", "closes", "starts"].includes(key as string)
                      ? "datetime-local"
                      : "text"
                  }
                  value={value as string}
                  onChange={(e) =>
                    (setter as (v: string) => void)(e.target.value)
                  }
                />
              </FormControl>
            ))}
            <FormControl>
              <FormLabel htmlFor="event-brief">
                {t("ConnectionFlow.briefFile")}
              </FormLabel>
              <Select
                id="event-brief"
                placeholder={t("ConnectionFlow.briefFile")}
                value={brief}
                onChange={(e) => setBrief(e.target.value)}
              >
                {Object.entries(data?.commits || {}).flatMap(
                  ([commit, value]) =>
                    Object.entries(value.files).map(([path, link]) => (
                      <option key={`${commit}/${path}`} value={link["/"]}>
                        {path} · {value.message} · {link["/"].slice(-8)}
                      </option>
                    ))
                )}
              </Select>
            </FormControl>
            <Button as={Link} href="/federation/files">
              {t("Office.upload")}
            </Button>
            <Button
              isDisabled={busy || !title || !brief || !validWindow}
              isLoading={busy && pendingKey === "events:create"}
              onClick={() =>
                void act(
                  {
                    type: "events.create",
                    id: crypto.randomUUID(),
                    title,
                    brief_hash: brief,
                    window: eventWindow,
                  },
                  "events:create"
                )
              }
            >
              {t("Office.create")}
            </Button>
          </VStack>
        </Section>
      )}
      {data && Object.keys(data.state.events).length === 0 && (
        <Empty description={t("Office.empty")} />
      )}
      {Object.entries(data?.state.events || {}).map(([id, event]) => (
        <EventCard key={id} id={id} event={event} />
      ))}
    </OfficePage>
  );
}

function EventCard({ id, event }: { id: string; event: OfficeEvent }) {
  const { t } = useTranslation();
  const { data, member, isAdmin } = useOffice();
  const request = useOfficeRequest();
  const { busy, pendingKey, act } = request;
  const [taskTitle, setTaskTitle] = useState("");
  const [template, setTemplate] = useState("");
  const templates = Object.entries(data?.state.flow_templates || {}).filter(
    ([, v]) =>
      v.active && v.versions.at(-1)?.payload_kinds.includes("events.approve")
  );
  const own = data?.identity.members[member || ""];
  const approvalTemplate = templates
    .find(([key]) => key === template)?.[1]
    .versions.at(-1);
  const needsFile = !!approvalTemplate?.stages.some(
    (stage) => stage.approver.type === "ai"
  );
  const input = Object.values(data?.commits || {}).some((commit) =>
    Object.values(commit.files).some((link) => link["/"] === event.brief_hash)
  )
    ? event.brief_hash
    : null;
  const manage =
    isAdmin ||
    event.creator === member ||
    (own?.club === event.organizer && own.roles.includes("president"));
  const active = event.phase !== "done" && event.phase !== "cancelled";
  const joined = !!member && !!data?.state.signups[id]?.includes(member);
  return (
    <AdvancedCard>
      <VStack align="stretch" spacing={3}>
        <HStack justify="space-between">
          <Text fontWeight="semibold">{event.title}</Text>
          <Tag>{t(`Office.status.${event.phase}`)}</Tag>
        </HStack>
        <Text>{event.organizer}</Text>
        <Text fontSize="sm">
          {t("Office.starts")}:{" "}
          {new Date(event.window.starts_ms).toLocaleString()}
        </Text>
        <HStack>
          <Text>{t("Office.brief")}</Text>
          <HashChip value={event.brief_hash} />
        </HStack>
        <HStack flexWrap="wrap">
          {manage &&
            [
              { phase: "draft", op: "submit" },
              { phase: "approved", op: "publish" },
              { phase: "open", op: "finish" },
            ]
              .filter((a) => a.phase === event.phase)
              .map((a) => (
                <Button
                  key={a.op}
                  isDisabled={busy}
                  isLoading={busy && pendingKey === `events:${id}:${a.op}`}
                  onClick={() =>
                    void act(
                      { type: `events.${a.op}`, id },
                      `events:${id}:${a.op}`
                    )
                  }
                >
                  {t(`Office.${a.op}`)}
                </Button>
              ))}
          {member && active && (joined || event.phase === "open") && (
            <Button
              isDisabled={busy}
              isLoading={busy && pendingKey === `events:${id}:signup`}
              onClick={() =>
                void act(
                  {
                    type: joined ? "events.cancel" : "events.signup",
                    id,
                  },
                  `events:${id}:signup`
                )
              }
            >
              {t(joined ? "Office.cancelSignup" : "Office.signup")}
            </Button>
          )}
          {manage && active && (
            <Button
              isDisabled={busy}
              isLoading={busy && pendingKey === `events:${id}:cancel_event`}
              onClick={() =>
                void act(
                  { type: "events.cancel_event", id },
                  `events:${id}:cancel_event`
                )
              }
            >
              {t("Office.cancelEvent")}
            </Button>
          )}
        </HStack>
        {manage && event.phase === "review" && (
          <HStack>
            <Select
              aria-label={t("Office.template")}
              placeholder={t("Office.template")}
              value={template}
              onChange={(e) => setTemplate(e.target.value)}
            >
              {templates.map(([key, v]) => (
                <option key={key} value={key}>
                  {v.versions.at(-1)?.name}
                </option>
              ))}
            </Select>
            <Button
              isDisabled={busy || !approvalTemplate || (needsFile && !input)}
              isLoading={busy && pendingKey === `events:${id}:requestApproval`}
              onClick={() =>
                void act(
                  {
                    type: "flows.start",
                    id: crypto.randomUUID(),
                    template,
                    payload: {
                      type: "events.approve",
                      id,
                      epoch: event.epoch,
                      co_organizers: event.co_organizers,
                    },
                    input,
                  },
                  `events:${id}:requestApproval`
                )
              }
            >
              {t("Office.requestApproval")}
            </Button>
          </HStack>
        )}
        {manage && event.phase === "review" && needsFile && !input && (
          <Text role="status">{t("Office.aiNeedsFile")}</Text>
        )}
        {member && joined && event.phase === "open" && (
          <Button
            isDisabled={busy || !!data?.state.checkins?.[id]?.includes(member)}
            isLoading={busy && pendingKey === `events:${id}:checkin`}
            onClick={() =>
              void act({ type: "events.checkin", id }, `events:${id}:checkin`)
            }
          >
            {t(
              data?.state.checkins?.[id]?.includes(member)
                ? "Office.checkedIn"
                : "Office.checkin"
            )}
          </Button>
        )}
        <Text fontSize="sm">
          {t("Office.signups", {
            count: data?.state.signups[id]?.length || 0,
          })}
        </Text>
        <Section title={t("Office.tasks")} isAccordion initialIsOpen={false}>
          <VStack align="stretch" spacing={3}>
            {Object.entries(data?.state.tasks || {})
              .filter(([, task]) => task.event === id)
              .map(([taskId, task]) => (
                <TaskRow
                  key={taskId}
                  taskId={taskId}
                  task={task}
                  event={event}
                  manage={manage}
                  active={active}
                />
              ))}
            {manage && active && (
              <HStack>
                <Input
                  aria-label={t("Office.taskTitle")}
                  placeholder={t("Office.taskTitle")}
                  value={taskTitle}
                  onChange={(e) => setTaskTitle(e.target.value)}
                />
                <Button
                  isDisabled={busy || !taskTitle}
                  isLoading={busy && pendingKey === `events:${id}:addTask`}
                  onClick={() =>
                    void act(
                      {
                        type: "events.add_task",
                        event: id,
                        id: crypto.randomUUID(),
                        title: taskTitle,
                        due_ms: null,
                      },
                      `events:${id}:addTask`
                    )
                  }
                >
                  {t("Office.addTask")}
                </Button>
              </HStack>
            )}
          </VStack>
        </Section>
      </VStack>
    </AdvancedCard>
  );
}

function TaskRow({
  taskId,
  task,
  event,
  manage,
  active,
}: {
  taskId: string;
  task: OfficeTask;
  event: OfficeEvent;
  manage: boolean;
  active: boolean;
}) {
  const { t } = useTranslation();
  const { data, member } = useOffice();
  const request = useOfficeRequest();
  const { busy, pendingKey, act } = request;
  const [assignee, setAssignee] = useState("");
  return (
    <VStack align="stretch">
      <Text>
        {task.title} ·{" "}
        {t(task.done ? "Office.status.done" : "Office.status.active")}
      </Text>
      {task.assignee && <Text fontSize="sm">{task.assignee}</Text>}
      <HStack>
        {manage && active && !task.done && (
          <>
            <Select
              aria-label={t("Office.assignee")}
              placeholder={t("Office.assignee")}
              value={assignee}
              onChange={(e) => setAssignee(e.target.value)}
            >
              {Object.entries(data?.identity.members || {})
                .filter(
                  ([, m]) =>
                    m.club === event.organizer ||
                    event.co_organizers.includes(m.club)
                )
                .map(([m]) => (
                  <option key={m} value={m}>
                    {m}
                  </option>
                ))}
            </Select>
            <Button
              isDisabled={busy || !assignee}
              isLoading={
                busy && pendingKey === `events:${task.event}:${taskId}:assign`
              }
              onClick={() =>
                void act(
                  {
                    type: "events.assign",
                    task: taskId,
                    member: assignee,
                  },
                  `events:${task.event}:${taskId}:assign`
                )
              }
            >
              {t("Office.assign")}
            </Button>
          </>
        )}
        {active && !task.done && (manage || task.assignee === member) && (
          <Button
            isDisabled={busy}
            isLoading={
              busy && pendingKey === `events:${task.event}:${taskId}:complete`
            }
            onClick={() =>
              void act(
                {
                  type: "events.complete",
                  task: taskId,
                },
                `events:${task.event}:${taskId}:complete`
              )
            }
          >
            {t("Office.complete")}
          </Button>
        )}
      </HStack>
    </VStack>
  );
}
