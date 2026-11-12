import { t } from "i18next";
import { InvokeResponse } from "@/models/response";
import { isDev } from "@/utils/env";
import { logger } from "@/utils/logging";

export function responseHandler(serviceDomain: string) {
  return function <Method>(
    _target: object,
    propertyKey: string | symbol,
    descriptor: TypedPropertyDescriptor<Method>
  ): TypedPropertyDescriptor<Method> {
    const originalMethod = descriptor.value as (
      ...args: unknown[]
    ) => Promise<unknown>;

    descriptor.value = async function (
      this: unknown,
      ...args: unknown[]
    ): Promise<InvokeResponse<unknown>> {
      try {
        const data = await originalMethod.apply(this, args);
        const message = t(
          `Services.${serviceDomain}.${String(propertyKey)}.success`
        );
        return { status: "success", message, data };
      } catch (error: unknown) {
        const message = t(
          `Services.${serviceDomain}.${String(propertyKey)}.error.title`
        );

        const details = String(error)
          ? t(
              `Services.${serviceDomain}.${String(propertyKey)}.error.description.${String(error)}`,
              {
                defaultValue: t("Services.unknownError"),
              }
            )
          : "";

        if (isDev) {
          const errorSet = {
            Key: String(error),
            "Service Domain": serviceDomain,
            "Property Key": String(propertyKey),
            "Localized Message": message,
            "Localized Details": details,
          };
          console.table(errorSet);
        }
        logger.error(
          `Invoke ${String(propertyKey)} failed: ${message} - ${details}`
        );

        return { status: "error", message, details, raw_error: String(error) };
      }
    } as Method;

    return descriptor;
  };
}
