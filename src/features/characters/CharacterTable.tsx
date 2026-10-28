import type { Character } from "../../shared/api/characters";

interface Props {
  characters: Character[];
  caption: string;
  showWorld?: boolean;
  selectedOcid?: string;
  onSelect?: (character: Character) => void;
  disabled?: boolean;
}

export default function CharacterTable({ characters, caption, showWorld, selectedOcid, onSelect, disabled }: Props) {
  return (
    <table>
      <caption className="visually-hidden">{caption}</caption>
      <thead><tr><th scope="col">캐릭터</th>{showWorld && <th scope="col">월드</th>}<th scope="col">직업</th><th scope="col">레벨</th>{onSelect && <th scope="col">본캐 지정</th>}</tr></thead>
      <tbody>{characters.map((character) => (
        <tr key={character.ocid}>
          <td>{character.character_name?.trim() || "이름 정보 없음"}{selectedOcid === character.ocid && <span className="main-badge">본캐</span>}</td>
          {showWorld && <td>{character.world_name}</td>}
          <td>{character.character_class?.trim() || "직업 정보 없음"}</td>
          <td>{character.character_level === null ? "정보 없음" : `Lv. ${character.character_level}`}</td>
          {onSelect && <td><button type="button" disabled={disabled || selectedOcid === character.ocid} onClick={() => onSelect(character)}>{selectedOcid === character.ocid ? "현재 본캐" : "본캐로 지정"}</button></td>}
        </tr>
      ))}</tbody>
    </table>
  );
}
