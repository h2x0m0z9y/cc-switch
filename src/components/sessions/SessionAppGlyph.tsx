import { AppGlyph } from "@/components/shell/AppGlyph";
import { cn } from "@/lib/utils";
import { SESSION_ONLY_APP_ICONS } from "./sessionOnlyIcons";
import { isAppId, type SessionAppId } from "./utils";

interface SessionAppGlyphProps {
  app: SessionAppId;
  size?: number;
  badgeClassName?: string;
  className?: string;
}

/**
 * 会话来源图标：完整应用用 `AppGlyph`；仅会话来源（CodeBuddy CN / Qoder / Qoder CN）
 * 使用各自 App 的真实图标。
 */
export function SessionAppGlyph({
  app,
  size = 16,
  badgeClassName,
  className,
}: SessionAppGlyphProps) {
  if (isAppId(app)) {
    return (
      <AppGlyph
        app={app}
        size={size}
        badgeClassName={badgeClassName}
        className={className}
      />
    );
  }
  return (
    <span
      aria-hidden="true"
      className={cn(
        "relative inline-flex shrink-0 items-center justify-center",
        className,
      )}
      style={{ width: size + 2, height: size + 2 }}
    >
      <img
        src={SESSION_ONLY_APP_ICONS[app]}
        alt=""
        draggable={false}
        className="object-contain"
        style={{ width: size, height: size }}
      />
    </span>
  );
}