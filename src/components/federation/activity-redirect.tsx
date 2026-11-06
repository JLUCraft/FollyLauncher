import { useRouter } from "next/router";
import { useEffect } from "react";
import { FEDERATION_ACTIVITIES_PATH } from "@/utils/federation/navigation";

export function ActivityRedirect({ page }: { page?: "event" | "competition" }) {
  const router = useRouter();

  useEffect(() => {
    if (!router.isReady) return;
    void router.replace({
      pathname: `${FEDERATION_ACTIVITIES_PATH}${page ? `/${page}` : ""}`,
      query: router.query,
    });
  }, [page, router]);

  return null;
}
