export enum ResourceType {
  Any,
  Sound,
  Graphics,
}

export function resourceTypeFromFilename(filename: string): ResourceType {
  const mapping: Record<string, ResourceType> = {
    wav: ResourceType.Sound,
    ogg: ResourceType.Sound,
    sti: ResourceType.Graphics,
  };
  const split = filename.split('.');
  if (split.length < 2) {
    return ResourceType.Any;
  }
  const extension = split[split.length - 1]?.toLowerCase();
  if (!extension) {
    return ResourceType.Any;
  }

  return mapping[extension] ?? ResourceType.Any;
}
