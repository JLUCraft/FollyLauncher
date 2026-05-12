import type { OnboardingStatus } from "../services/account";

export function canCreateFederatedRoom(status: OnboardingStatus): boolean {
  return status.is_member === true;
}

export function roomCreationBlockedMessage(
  status: OnboardingStatus,
): string | null {
  if (canCreateFederatedRoom(status)) return null;
  if (status.is_guest) {
    return "创建房间需要社团 VC；MUA 访客仅可加入公开/MUA 实例。";
  }
  return "创建房间需要社团 VC；请联系社长申请平台身份。";
}