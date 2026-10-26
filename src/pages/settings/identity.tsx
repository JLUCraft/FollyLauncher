import { useRouter } from "next/router";
import { useEffect } from "react";
import { FEDERATION_SETTINGS_PATH } from "@/utils/federation/navigation";

export default function IdentitySettingsRedirect() {
  const router = useRouter();
  useEffect(() => {
    void router.replace({
      pathname: FEDERATION_SETTINGS_PATH,
      query: router.query,
    });
  }, [router]);
  return null;
}
