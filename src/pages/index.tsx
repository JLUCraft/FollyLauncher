import { useRouter } from "next/router";
import { useEffect } from "react";

const IndexPage = () => {
  const router = useRouter();

  useEffect(() => {
    router.push("/launch").catch((error: unknown) => console.error(error));
  }, [router]);

  return null;
};

export default IndexPage;
