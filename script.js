const ua = navigator?.userAgent.toLowerCase();
// const os = 'unknown';
const os = ua.includes('win') ? 'windows'
  : ua.includes('mac') ? 'macos'
  : ua.includes('linux') ? 'linux'
  : 'unknown';

const kicad_wakatime_version = document.getElementById('kicad-wakatime-version').innerHTML;

const e_download_kicad = document.getElementById('download-kicad');
const e_download_kicad_wakatime = document.getElementById('download-kicad-wakatime');
const e_download_kicad_href = os == 'windows' ? 'https://www.kicad.org/download/windows'
  : os == 'macos' ? 'https://www.kicad.org/download/macos'
  : os == 'linux' ? 'https://www.kicad.org/download/linux'
  : null;
const e_download_kicad_wakatime_href = os == 'windows' ? 'https://github.com/hackclub/kicad-wakatime/releases/download/0.3.0/kicad-wakatime-0.3.0-windows.zip'
  : os == 'macos' ? 'https://github.com/hackclub/kicad-wakatime/releases/download/0.3.0/kicad-wakatime-0.3.0-macos.zip'
  : os == 'linux' ? 'https://github.com/hackclub/kicad-wakatime/releases/download/0.3.0/kicad-wakatime-0.3.0-linux.zip'
  : 'https://github.com/hackclub/kicad-wakatime/releases/tag/0.3.0'

document.getElementById('os').innerHTML = os;

e_download_kicad.innerHTML = os == 'windows' ? `<a href="${e_download_kicad_href}" target="_blank">download KiCAD (.exe)</a>`
  : os == 'macos' ? `<a href="${e_download_kicad_href}" target="_blank">download KiCAD (.dmg)</a>`
  : os == 'linux' ? `<a href="${e_download_kicad_href}" target="_blank">download KiCAD (instructions)</a>`
  : `<a href="https://www.kicad.org/download" target="_blank">download KiCAD</a>`;

e_download_kicad_wakatime.innerHTML = `<a href="${e_download_kicad_wakatime_href}" target="_blank">download kicad-wakatime (.zip)</a>`;
