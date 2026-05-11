import type { OnboardingStatus } from "../services/account";

/**
 * 判断当前身份是否允许创建联邦房间。
 * 规则：仅 VC Member（is_member === true）允许。
 */
export function canCreateFederatedRoom(status: OnboardingStatus): boolean {
  return status.is_member === true;
}

/**
 * 当不允许创建联邦房间时，返回面向玩家的提示文案。
 * 不同身份返回不同引导文案。
 * 返回 null 表示允许创建。
 */
export function roomCreationBlockedMessage(
  status: OnboardingStatus,
): string | null {
  if (canCreateFederatedRoom(status)) return null;
  if (status.is_guest) {
    return "创建房间需要社团 VC；MUA 访客仅可加入公开/MUA 实例。";
  }
  return "创建房间需要社团 VC；请联系社长申请平台身份。";
}
