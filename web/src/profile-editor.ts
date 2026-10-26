/** Reorders ordered payload/range/desync steps while retaining filter argument positions. */
export function reorderProfileSteps(args: readonly string[], stepIndex: number, direction: -1 | 1): string[] {
  const isOrderedStep = (argument: string) => argument.startsWith('--lua-desync=')
    || argument.startsWith('--payload=')
    || argument.startsWith('--in-range=')
    || argument.startsWith('--out-range=');
  const positions = args.flatMap((argument, index) => isOrderedStep(argument) ? [index] : []);
  const targetStepIndex = stepIndex + direction;
  if (stepIndex < 0 || targetStepIndex < 0 || stepIndex >= positions.length || targetStepIndex >= positions.length) return [...args];

  const result = [...args];
  const from = positions[stepIndex];
  const to = positions[targetStepIndex];
  if (from === undefined || to === undefined) return result;
  [result[from], result[to]] = [result[to]!, result[from]!];
  return result;
}

export function replaceDesyncFunction(argument: string, functionName: string): string {
  const prefix = '--lua-desync=';
  if (!argument.startsWith(prefix) || !/^[a-zA-Z0-9_-]+$/.test(functionName)) return argument;
  const [, ...parameters] = argument.slice(prefix.length).split(':');
  return `${prefix}${functionName}${parameters.length ? `:${parameters.join(':')}` : ''}`;
}

export function replaceDesyncParameters(argument: string, parameters: string): string {
  const prefix = '--lua-desync=';
  if (!argument.startsWith(prefix)) return argument;
  const [functionName] = argument.slice(prefix.length).split(':');
  if (!functionName) return argument;
  const value = parameters.trim().replace(/^:+/, '');
  return `${prefix}${functionName}${value ? `:${value}` : ''}`;
}

export function replaceProfileName(args: readonly string[], name: string): string[] {
  const value = name.trim();
  if (value && (!/^[\p{L}\p{N}_.-]{1,64}$/u.test(value))) return [...args];
  const result = args.filter((argument) => !argument.startsWith('--name='));
  if (value) result.unshift(`--name=${value}`);
  return result;
}

export function replaceProfileArgument(args: readonly string[], index: number, argument: string): string[] {
  if (!Number.isInteger(index) || index < 0 || index >= args.length) return [...args];
  if (!argument || /\s|[\0\r\n'"`$\\;&|]/.test(argument) || argument === '--new' || argument.startsWith('--new=')) return [...args];
  const result = [...args];
  result[index] = argument;
  return result;
}
