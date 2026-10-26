import { useRouter } from "next/router";
import { useEffect } from "react";
import { FEDERATION_ROOMS_PATH } from "@/utils/federation/navigation";

export default function RoomsRedirect() {
  const router = useRouter();
  useEffect(() => {
    void router.replace(FEDERATION_ROOMS_PATH);
  }, [router]);
  return null;
}
