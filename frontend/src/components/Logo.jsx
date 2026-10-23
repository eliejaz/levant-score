export default function Logo({ light = false }) {
  return <div className={`logo ${light ? 'logo-light' : ''}`} aria-label="Levant Score">
    <span className="logo-mark"><i>♥</i><i>♠</i></span>
    <span>Levant <b>Score</b></span>
  </div>
}

