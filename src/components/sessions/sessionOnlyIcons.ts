import codebuddyCn from "@/assets/app-icons/codebuddy-cn.png";
import qoder from "@/assets/app-icons/qoder.png";
import qoderCn from "@/assets/app-icons/qoder-cn.png";
import type { SessionOnlyAppId } from "./utils";

/**
 * 仅会话来源的真实 App 图标（取自各自 `.app` 包内的 `.icns`，转成 256px PNG）。
 * 这些来源不是 `AppId`，没有侧栏图标，会话管理器单独使用这里的资源。
 */
export const SESSION_ONLY_APP_ICONS: Record<SessionOnlyAppId, string> = {
  "codebuddy-cn": codebuddyCn,
  qoder,
  "qoder-cn": qoderCn,
};