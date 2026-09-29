import { useRouter } from "next/router";
import { useEffect } from "react";

export default function FederationIndex() {
  const router = useRouter();
  useEffect(() => {
    void router.replace("/federation/rooms");
  }, [router]);
  return null;
}
