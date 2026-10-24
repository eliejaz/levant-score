const MODE_KEY='levant-storage-mode'

export function getStorageMode(){
  const mode=localStorage.getItem(MODE_KEY)
  return mode==='local'||mode==='api'?mode:null
}

export function setStorageMode(mode){
  if(mode==='local'||mode==='api')localStorage.setItem(MODE_KEY,mode)
  else localStorage.removeItem(MODE_KEY)
}


