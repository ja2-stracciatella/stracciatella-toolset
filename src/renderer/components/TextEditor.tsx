import { useFileEditMode, useFileText } from '../hooks/files';

export interface TextEditorProps {
  file: string;
}

export function TextEditor({ file }: TextEditorProps) {
  const [text] = useFileText(file);
  return <div>{text}</div>;
}

export function TextEditorOr({
  file,
  children,
}: TextEditorProps & { children?: React.ReactNode }) {
  const editMode = useFileEditMode(file);

  if (editMode === 'text') {
    return <TextEditor file={file} />;
  }
  return children;
}
