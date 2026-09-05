// Console honors the explicit IDs/globs in Pi's enabledModels cycling preference.
// This is a picker preference, not an authorization boundary.
export function inModelScope(provider: string, id: string, patterns: string[] = []): boolean {
  if (!patterns.length) return true;
  return patterns.some(pattern => {
    const escaped = pattern.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.');
    const regex = new RegExp(`^${escaped}$`, 'i');
    return regex.test(`${provider}/${id}`) || regex.test(id);
  });
}
