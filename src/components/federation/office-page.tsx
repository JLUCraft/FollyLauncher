import { Button, Text, VStack } from "@chakra-ui/react";
import Link from "next/link";
import { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import Empty from "@/components/common/empty";
import { Section } from "@/components/common/section";
import {
  OFFICE_REFRESH_KEY,
  useOffice,
  useOfficeRequest,
} from "@/contexts/office";

export function OfficeFeedback({
  request,
}: {
  request: Pick<ReturnType<typeof useOfficeRequest>, "error" | "uncertain">;
}) {
  const { t } = useTranslation();
  return request.error ? (
    <Text role="alert" color="red.500">
      {t(request.uncertain ? "Office.uncertain" : "Office.error")}
    </Text>
  ) : null;
}
export function OfficePage({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const { ready, connecting, connectionError, data } = useOffice();
  const refresh = useOfficeRequest();
  return (
    <Section title={title}>
      <VStack align="stretch" spacing={4}>
        {connecting && (
          <Text role="status">{t("ConnectionFlow.connecting")}</Text>
        )}
        {!ready ? (
          <>
            <Empty description={t("Office.setup")} />
            <Button as={Link} href="/federation/overview">
              {t("ConnectionFlow.setup")}
            </Button>
          </>
        ) : (
          <Button
            isLoading={
              (refresh.busy && refresh.pendingKey === OFFICE_REFRESH_KEY) ||
              connecting
            }
            onClick={() => void refresh.run(undefined, OFFICE_REFRESH_KEY)}
          >
            {t("Office.refresh")}
          </Button>
        )}
        {connectionError && !data && (
          <Text role="alert">{t("Office.error")}</Text>
        )}
        <OfficeFeedback request={refresh} />
        {ready &&
          !data &&
          !connecting &&
          !refresh.busy &&
          !connectionError &&
          !refresh.error && <Empty description={t("Office.initial")} />}
        {data && children}
      </VStack>
    </Section>
  );
}
