// Input/instance generations, including pending debounce work, fail closed.
export function createSubscriptionPreview({query,delay=250,schedule=setTimeout,cancel=clearTimeout}) {
  let generation=0, timer=null, disposed=false;
  const invalidate=()=>{++generation;if(timer!==null){cancel(timer);timer=null;}};
  return {
    invalidate,
    request(context,suffix,accept,reject){
      if(disposed)return;invalidate();const token=generation;
      timer=schedule(async()=>{
        timer=null;
        if(disposed||token!==generation)return;
        try{const result=await query({canisterId:context,suffix});if(!disposed&&token===generation)accept(result);}
        catch(error){if(!disposed&&token===generation)reject(error);}
      },delay);
    },
    dispose(){invalidate();disposed=true;},
  };
}
