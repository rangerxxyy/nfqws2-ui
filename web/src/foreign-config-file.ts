const SUPPORTED_FOREIGN_CONFIG_EXTENSIONS = ['.conf', '.txt'];

export function isSupportedForeignConfigFileName(name: string): boolean {
  const normalizedName = name.trim().toLowerCase();
  return SUPPORTED_FOREIGN_CONFIG_EXTENSIONS.some((extension) => normalizedName.endsWith(extension));
}
